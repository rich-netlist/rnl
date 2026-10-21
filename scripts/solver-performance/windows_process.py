"""Measure an owned child process; Windows CPU times and observed peak RSS, no extra process."""
import ctypes
import os
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from ctypes import wintypes

class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [(name, ctypes.c_size_t) for name in ("PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage", "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage")]

def _unbounded_run(command, env):
    system_times = None
    if os.name == "nt":
        get_system_times = ctypes.WinDLL("kernel32", use_last_error=True).GetSystemTimes
        get_system_times.argtypes = [ctypes.POINTER(wintypes.FILETIME)] * 3
        get_system_times.restype = wintypes.BOOL
        def system_snapshot():
            values = [wintypes.FILETIME() for _ in range(3)]
            if not get_system_times(*(ctypes.byref(value) for value in values)): return None
            return [((value.dwHighDateTime << 32) + value.dwLowDateTime) / 10_000_000 for value in values]
        system_times = system_snapshot()
    started = time.perf_counter()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    peak = 0
    if os.name == "nt":
        handle = wintypes.HANDLE(int(process._handle))
        memory_info = ctypes.WinDLL("psapi", use_last_error=True).GetProcessMemoryInfo
        memory_info.argtypes = [wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD]
        memory_info.restype = wintypes.BOOL
    while True:
        if os.name == "nt":
            memory = MemoryCounters(); memory.cb = ctypes.sizeof(memory)
            if memory_info(handle, ctypes.byref(memory), memory.cb): peak = max(peak, memory.PeakWorkingSetSize)
        try:
            stdout, stderr = process.communicate(timeout=0.25)
            break
        except subprocess.TimeoutExpired:
            pass
    result = {"child_pid": process.pid, "cpu_seconds": None, "peak_working_set_observed_bytes": peak or None, "memory_sampling_seconds": 0.25}
    if os.name == "nt":
        times = [wintypes.FILETIME() for _ in range(4)]
        get_times = ctypes.WinDLL("kernel32", use_last_error=True).GetProcessTimes
        get_times.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
        get_times.restype = wintypes.BOOL
        if get_times(handle, *(ctypes.byref(value) for value in times)):
            ticks = lambda value: (value.dwHighDateTime << 32) + value.dwLowDateTime
            result["cpu_seconds"] = (ticks(times[2]) + ticks(times[3])) / 10_000_000
        else: result["cpu_measurement_error"] = ctypes.get_last_error()
        after = system_snapshot()
        if system_times and after:
            idle, kernel, user = [last - first for first, last in zip(system_times, after)]
            total = kernel + user
            busy = max(0.0, total - idle)
            result.update(system_total_cpu_seconds=total, system_idle_cpu_seconds=idle,
                          system_busy_cpu_seconds=busy,
                          approximate_external_busy_cpu_seconds=max(0.0, busy - result["cpu_seconds"]) if result["cpu_seconds"] is not None else None,
                          system_utilization=busy / total if total else None)
    return subprocess.CompletedProcess(command, process.returncode, stdout, stderr), time.perf_counter() - started, result


class OwnershipError(RuntimeError):
    pass


def _utc():
    return datetime.now(timezone.utc).isoformat()


def _same_identity(expected, actual):
    for field in ('pid', 'creation_filetime', 'exe'):
        if expected.get(field) != actual.get(field):
            raise OwnershipError('owned process identity differs: ' + field)
    if actual.get('parent_pid') is not None and expected.get('parent_pid') != actual['parent_pid']:
        raise OwnershipError('owned process parent differs')


def _completion_status(execution_seconds, deadline_seconds):
    return 'COMPLETED' if execution_seconds <= deadline_seconds else 'LATE_RETURN'


def _system_snapshot():
    if os.name != 'nt':
        return None
    method = ctypes.WinDLL('kernel32', use_last_error=True).GetSystemTimes
    method.argtypes = [ctypes.POINTER(wintypes.FILETIME)] * 3
    method.restype = wintypes.BOOL
    values = [wintypes.FILETIME() for _ in range(3)]
    if not method(*(ctypes.byref(value) for value in values)):
        return None
    return [((value.dwHighDateTime << 32) + value.dwLowDateTime) / 10_000_000 for value in values]


class _OwnedJob:
    """One unnamed, non-breakaway Windows Job; root is assigned before its first instruction."""
    def __init__(self):
        if os.name != 'nt':
            raise OwnershipError('bounded native execution is currently Windows-only')
        self.kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        self.job = None
        self.process = None
        self.expected = None
        k = self.kernel
        def api(name, args, result):
            value = getattr(k, name)
            value.argtypes, value.restype = args, result
            return value
        H, D, B, P = wintypes.HANDLE, wintypes.DWORD, wintypes.BOOL, ctypes.c_void_p
        self.create_job = api('CreateJobObjectW', [P, wintypes.LPCWSTR], H)
        self.close = api('CloseHandle', [H], B)
        self.set_job = api('SetInformationJobObject', [H, ctypes.c_int, P, D], B)
        self.query_job = api('QueryInformationJobObject', [H, ctypes.c_int, P, D, ctypes.POINTER(D)], B)
        self.assign = api('AssignProcessToJobObject', [H, H], B)
        self.in_job = api('IsProcessInJob', [H, H, ctypes.POINTER(B)], B)
        self.terminate = api('TerminateJobObject', [H, wintypes.UINT], B)
        self.process_id = api('GetProcessId', [H], D)
        self.times = api('GetProcessTimes', [H] + [ctypes.POINTER(wintypes.FILETIME)] * 4, B)
        self.image = api('QueryFullProcessImageNameW', [H, D, wintypes.LPWSTR, ctypes.POINTER(D)], B)
        self.snapshot = api('CreateToolhelp32Snapshot', [D, D], H)
        self.open_process = api('OpenProcess', [D, B, D], H)
        self.open_thread = api('OpenThread', [D, B, D], H)
        self.thread_pid = api('GetProcessIdOfThread', [H], D)
        self.resume = api('ResumeThread', [H], D)
        class BasicLimits(ctypes.Structure):
            _fields_ = [('process_time', ctypes.c_int64), ('job_time', ctypes.c_int64), ('flags', D),
                ('min_working_set', ctypes.c_size_t), ('max_working_set', ctypes.c_size_t),
                ('active_limit', D), ('affinity', ctypes.c_size_t), ('priority', D), ('scheduling', D)]
        class IO(ctypes.Structure):
            _fields_ = [(name, ctypes.c_uint64) for name in ('read_ops', 'write_ops', 'other_ops', 'read_bytes', 'write_bytes', 'other_bytes')]
        class ExtendedLimits(ctypes.Structure):
            _fields_ = [('basic', BasicLimits), ('io', IO), ('process_memory', ctypes.c_size_t),
                ('job_memory', ctypes.c_size_t), ('peak_process_memory', ctypes.c_size_t), ('peak_job_memory', ctypes.c_size_t)]
        self.job = self.create_job(None, None)
        if not self.job:
            raise OwnershipError('CreateJobObject: ' + str(ctypes.get_last_error()))
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE; no breakaway flags.
        if not self.set_job(self.job, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close(self.job)
            self.job = None
            raise OwnershipError('SetInformationJobObject: ' + str(ctypes.get_last_error()))

    def parents(self):
        D = wintypes.DWORD
        class ProcessEntry(ctypes.Structure):
            _fields_ = [('size', D), ('usage', D), ('pid', D), ('heap', ctypes.c_size_t),
                ('module', D), ('threads', D), ('parent', D), ('priority', wintypes.LONG), ('flags', D), ('exe', wintypes.WCHAR * 260)]
        handle = self.snapshot(2, 0)
        if handle == ctypes.c_void_p(-1).value:
            raise OwnershipError('process snapshot failed')
        first, following = self.kernel.Process32FirstW, self.kernel.Process32NextW
        for method in (first, following):
            method.argtypes, method.restype = [wintypes.HANDLE, ctypes.POINTER(ProcessEntry)], wintypes.BOOL
        result = {}
        entry = ProcessEntry()
        entry.size = ctypes.sizeof(entry)
        try:
            ok = first(handle, ctypes.byref(entry))
            while ok:
                result[entry.pid] = entry.parent
                ok = following(handle, ctypes.byref(entry))
        finally:
            self.close(handle)
        return result

    def identity(self, handle, parents):
        pid = self.process_id(handle)
        values = [wintypes.FILETIME() for _ in range(4)]
        path = ctypes.create_unicode_buffer(32768)
        length = wintypes.DWORD(len(path))
        if not pid or not self.times(handle, *(ctypes.byref(v) for v in values)) or not self.image(handle, 0, path, ctypes.byref(length)):
            raise OwnershipError('owned PID/start/exe query failed: ' + str(ctypes.get_last_error()))
        created = (values[0].dwHighDateTime << 32) + values[0].dwLowDateTime
        return {'pid': int(pid), 'creation_filetime': created,
            'exe': os.path.normcase(os.path.realpath(path.value)), 'parent_pid': parents.get(pid)}

    def start(self, command, env, stdout, stderr):
        self.command_started_utc = _utc()
        self.command_started = time.perf_counter()
        self.process = subprocess.Popen(command, env=env, stdout=stdout, stderr=stderr,
            creationflags=0x00000004 | subprocess.CREATE_NEW_PROCESS_GROUP)  # CREATE_SUSPENDED
        handle = wintypes.HANDLE(int(self.process._handle))
        self.expected = self.identity(handle, self.parents())
        if self.expected['pid'] != self.process.pid or self.expected['parent_pid'] != os.getpid():
            raise OwnershipError('suspended root is not this runner child')
        if self.expected['exe'] != os.path.normcase(os.path.realpath(command[0])):
            raise OwnershipError('suspended root executable differs from the exact command')
        if not self.assign(self.job, handle):
            raise OwnershipError('AssignProcessToJobObject: ' + str(ctypes.get_last_error()))
        owned = wintypes.BOOL()
        if not self.in_job(handle, self.job, ctypes.byref(owned)) or not owned.value:
            raise OwnershipError('suspended root was not assigned to the private Job')
        # Popen closes its initial thread handle; open only this still-suspended root's unique thread.
        D = wintypes.DWORD
        class ThreadEntry(ctypes.Structure):
            _fields_ = [('size', D), ('usage', D), ('tid', D), ('pid', D), ('priority', wintypes.LONG), ('delta', wintypes.LONG), ('flags', D)]
        snapshot = self.snapshot(4, 0)
        if snapshot == ctypes.c_void_p(-1).value:
            raise OwnershipError('thread snapshot failed')
        first, following = self.kernel.Thread32First, self.kernel.Thread32Next
        for method in (first, following):
            method.argtypes, method.restype = [wintypes.HANDLE, ctypes.POINTER(ThreadEntry)], wintypes.BOOL
        entry = ThreadEntry()
        entry.size = ctypes.sizeof(entry)
        threads = []
        try:
            ok = first(snapshot, ctypes.byref(entry))
            while ok:
                if entry.pid == self.process.pid:
                    threads.append(entry.tid)
                ok = following(snapshot, ctypes.byref(entry))
        finally:
            self.close(snapshot)
        if len(threads) != 1:
            raise OwnershipError('suspended root does not have exactly one initial thread')
        thread = self.open_thread(0x0002 | 0x0800, False, threads[0])
        if not thread:
            raise OwnershipError('OpenThread failed')
        try:
            if self.thread_pid(thread) != self.process.pid:
                raise OwnershipError('initial thread belongs to another process')
            self.execution_started_utc = _utc()
            self.execution_started = time.perf_counter()
            if self.resume(thread) != 1:
                raise OwnershipError('initial thread did not resume from exactly one suspension')
        finally:
            self.close(thread)

    def members(self):
        buffer = ctypes.create_string_buffer(8 + ctypes.sizeof(ctypes.c_size_t) * 4096)
        if not self.query_job(self.job, 3, buffer, len(buffer), None):
            raise OwnershipError('private Job member query failed: ' + str(ctypes.get_last_error()))
        count = ctypes.cast(ctypes.byref(buffer, 4), ctypes.POINTER(wintypes.DWORD)).contents.value
        return list((ctypes.c_size_t * count).from_buffer(buffer, 8))

    def verify_tree(self):
        parents = self.parents()
        handle = wintypes.HANDLE(int(self.process._handle))
        try:
            root = self.identity(handle, parents)
            _same_identity(self.expected, root)
        except OwnershipError:
            if self.process.poll() is None:
                raise
            # QueryFullProcessImageName can fail after natural root exit. Its held
            # process object cannot be a reused PID. Recheck PID/creation on that
            # exact handle and record the missing *current* path query honestly.
            values = [wintypes.FILETIME() for _ in range(4)]
            if self.process_id(handle) != self.expected['pid'] or not self.times(handle, *(ctypes.byref(v) for v in values)):
                raise OwnershipError('exited root handle PID/start query failed')
            created = (values[0].dwHighDateTime << 32) + values[0].dwLowDateTime
            if created != self.expected['creation_filetime']:
                raise OwnershipError('exited root handle creation time differs')
            root = dict(pid=self.expected['pid'], creation_filetime=created,
                exe=None, exe_query='unavailable after confirmed root exit',
                exe_at_launch=self.expected['exe'], parent_pid=parents.get(self.expected['pid']),
                actual_exit_code=self.process.returncode,
                ownership='same held process object; active descendants are rechecked as private Job members')
        identities = []
        for pid in self.members():
            handle = self.open_process(0x1000 | 0x00100000, False, pid)
            if not handle:
                # It may have naturally exited; never kill a PID we cannot verify.
                if pid not in self.members():
                    continue
                raise OwnershipError('cannot open an active private Job member')
            try:
                actual = self.identity(handle, parents)
                in_job = wintypes.BOOL()
                if not self.in_job(handle, self.job, ctypes.byref(in_job)) or not in_job.value:
                    raise OwnershipError('PID was reused outside the private Job')
                if actual['creation_filetime'] < self.expected['creation_filetime']:
                    raise OwnershipError('private Job member predates the owned root')
                actual['ownership'] = 'suspended-root-assignment or non-breakaway private-Job inheritance'
                identities.append(actual)
            finally:
                self.close(handle)
        return {'root_rechecked': root, 'members': identities, 'checked_utc': _utc()}

    def stop(self):
        proof = self.verify_tree()  # Must succeed before terminating the owned Job.
        # RNL exposes no cooperative cancellation channel. Do not signal a numeric
        # console group PID: an exiting root can race PID reuse. A tiny natural-exit
        # grace is cleanup time, never extra accepted solve time, then use our Job handle.
        cancellation = {'requested_utc': _utc(), 'method': 'no cooperative channel; bounded natural-exit grace then private Job handle',
            'grace_seconds': 0.05, 'numeric_pid_signal_sent': False}
        until = time.perf_counter() + 0.05
        while self.members() and time.perf_counter() < until:
            time.sleep(min(0.005, max(0, until - time.perf_counter())))
        if self.members():
            proof = self.verify_tree()
            if not self.terminate(self.job, 1460):
                raise OwnershipError('TerminateJobObject failed: ' + str(ctypes.get_last_error()))
            cancellation['forced_utc'] = _utc()
        return proof, cancellation

    def cleanup(self):
        # Structural setup failure never falls back to an unowned running child. The held
        # Popen process handle is ours even when a later identity-query API failed.
        if self.job:
            self.close(self.job)
            self.job = None
        if self.process is not None and self.process.poll() is None:
            self.process.kill()
        if self.process is not None:
            self.process.wait(timeout=2)


def measured_run(command, env, *, deadline_seconds=None, stdout_path=None, stderr_path=None):
    if deadline_seconds is None:
        return _unbounded_run(command, env)
    if not 0 < deadline_seconds <= 120:
        raise ValueError('bounded deadline must be positive and at most 120 seconds')
    if stdout_path is None or stderr_path is None:
        raise ValueError('bounded execution requires owned persistent stdout/stderr paths')
    command = [str(Path(command[0]).resolve(strict=True)), *map(str, command[1:])]
    stdout_path, stderr_path = Path(stdout_path), Path(stderr_path)
    launched = time.perf_counter()
    result = {'execution_status': 'HARNESS_ERROR', 'deadline_seconds': deadline_seconds,
        'launch_started_utc': _utc(), 'timing_scope': 'CreateProcess-through-owned-tree-exit; includes owned startup and CLI, not pure library solve',
        'compile_and_environment_preparation_included': False, 'child_pid': None,
        'cpu_seconds': None, 'peak_working_set_observed_bytes': None, 'memory_sampling_seconds': 0.25}
    owner = None
    system_before = _system_snapshot()
    try:
        with stdout_path.open('wb') as stdout, stderr_path.open('wb') as stderr:
            owner = _OwnedJob()
            owner.start(command, env, stdout, stderr)
            process = owner.process
            memory_info = ctypes.WinDLL('psapi', use_last_error=True).GetProcessMemoryInfo
            memory_info.argtypes = [wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD]
            memory_info.restype = wintypes.BOOL
            next_sample, peak = owner.execution_started, 0
            result.update(child_pid=process.pid, ownership=owner.expected,
                execution_started_utc=owner.execution_started_utc,
                command_started_utc=owner.command_started_utc,
                process_setup_seconds=owner.execution_started - owner.command_started)
            deadline = owner.command_started + deadline_seconds
            while True:
                now = time.perf_counter()
                if now >= next_sample:
                    memory = MemoryCounters()
                    memory.cb = ctypes.sizeof(memory)
                    if memory_info(wintypes.HANDLE(int(process._handle)), ctypes.byref(memory), memory.cb):
                        peak = max(peak, memory.PeakWorkingSetSize)
                    next_sample = now + 0.25
                if process.poll() is not None and not owner.members():
                    result['execution_seconds'] = time.perf_counter() - owner.execution_started
                    result['command_seconds'] = time.perf_counter() - owner.command_started
                    result['execution_status'] = _completion_status(result['command_seconds'], deadline_seconds)
                    result['tree_empty_confirmed'] = True
                    break
                if now >= deadline:
                    result.update(execution_status='TIMEOUT', deadline_observed_utc=_utc(),
                        deadline_observed_seconds=now - owner.command_started)
                    proof, cancellation = owner.stop()
                    result.update(termination_ownership_recheck=proof, cancellation=cancellation)
                    cleanup_until = time.perf_counter() + 2
                    while owner.members() and time.perf_counter() < cleanup_until:
                        time.sleep(0.005)
                    result['tree_empty_confirmed'] = not owner.members()
                    result['execution_seconds'] = time.perf_counter() - owner.execution_started
                    result['command_seconds'] = time.perf_counter() - owner.command_started
                    result['termination_delay_seconds'] = max(0, result['command_seconds'] - deadline_seconds)
                    if not result['tree_empty_confirmed']:
                        raise OwnershipError('owned tree did not exit; do not dispatch another case')
                    process.wait(timeout=2)
                    break
                if process.poll() is None:
                    try:
                        process.wait(timeout=min(0.025, deadline - now))
                    except subprocess.TimeoutExpired:
                        pass
                else:
                    time.sleep(min(0.005, deadline - now))
            result['execution_finished_utc'] = _utc()
            result['actual_exit_code'] = process.returncode
            result['peak_working_set_observed_bytes'] = peak or None
            times = [wintypes.FILETIME() for _ in range(4)]
            if owner.times(wintypes.HANDLE(int(process._handle)), *(ctypes.byref(v) for v in times)):
                ticks = lambda value: (value.dwHighDateTime << 32) + value.dwLowDateTime
                result['cpu_seconds'] = (ticks(times[2]) + ticks(times[3])) / 10_000_000
    except Exception as error:
        result.update(execution_status='HARNESS_ERROR', harness_error=f'{type(error).__name__}: {error}',
            execution_finished_utc=_utc())
    finally:
        if owner is not None:
            if owner.process is not None and result['child_pid'] is None:
                result['child_pid'] = owner.process.pid
                result['ownership'] = owner.expected
            try:
                owner.cleanup()
                result['owned_cleanup_completed'] = True
                result['cleanup_finished_utc'] = _utc()
            except Exception as error:
                result.update(execution_status='HARNESS_ERROR', owned_cleanup_completed=False, cleanup_error=str(error))
    elapsed = time.perf_counter() - launched
    system_after = _system_snapshot()
    if system_before and system_after:
        idle, kernel, user = [last - first for first, last in zip(system_before, system_after)]
        total = kernel + user
        busy = max(0.0, total - idle)
        result.update(system_total_cpu_seconds=total, system_idle_cpu_seconds=idle, system_busy_cpu_seconds=busy,
            approximate_external_busy_cpu_seconds=max(0.0, busy - result['cpu_seconds']) if result['cpu_seconds'] is not None else None,
            system_utilization=busy / total if total else None)
    code = owner.process.returncode if owner is not None and owner.process is not None else None
    result.update(actual_exit_code=code, process_wall_seconds=elapsed,
        performance_pass=None, pure_library_solve_seconds=None,
        complete_result_assessment='requires downstream complete-result validation')
    return subprocess.CompletedProcess(command, code, stdout_path.read_bytes(), stderr_path.read_bytes()), elapsed, result
