"""Summarize collected data; never launch a solver or turn diagnostic completion into a pass."""
import argparse
import os
from pathlib import Path
import benchmark as b


def key(pair):
    item=pair['item']
    return ('representative',item['case_id']) if 'case_id' in item else ('synthetic',item['family'],item['n'])


def solved_folder(root, label, pair):
    item=pair['item']
    group='representative' if 'case_id' in item else 'synthetic'
    name=item.get('case_id',item.get('family','')+'/'+str(item.get('n','')))
    return root/label/'runs'/group/name.replace('/','__')/'off/solved'


def exact_outputs_equal(root, left, right):
    a,c=solved_folder(root,'baseline',left),solved_folder(root,'candidate',right)
    return b.actual_bytes_equal(a,c)


def work(pair):
    return [{k:level.get(k) for k in ('level_id','objects','facts','metrics','options_canonical','problem_hash')} for level in pair['on_level_observations']]


def transition(left, right):
    if left is None:
        return 'candidate_only_measured'
    if right is None:
        return 'baseline_only_measured'
    # within a fixed work a point is compared once both complete, whether or not within 10 s
    if left['off_status']==right['off_status']=='COMPLETE':
        return 'common_points'
    if left['off_status']=='TIMEOUT' and right['off_status']=='COMPLETE':
        return 'newly_completed_off_points'
    if left['off_status']=='COMPLETE' and right['off_status']=='TIMEOUT':
        return 'regressed_off_points'
    return 'other_observed_state_transitions'


def pressure(pair):
    return pair.get('off_resource_pressure_observed') is True or pair.get('on_resource_pressure_observed') is True


def clean_comparison(left, right):
    return not pressure(left) and not pressure(right) and bool(left.get('off_options_canonical')) and left['off_options_canonical']==right.get('off_options_canonical')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--artifact',type=Path,required=True)
    args=parser.parse_args();root=args.artifact
    summaries={label:b.load(root/label/'runs/summary.json') for label in ('baseline','candidate') if (root/label/'runs/summary.json').exists()}
    result={'collections':summaries,'common_points':[],'newly_completed_off_points':[],'regressed_off_points':[],
            'candidate_only_measured':[],'baseline_only_measured':[],'other_observed_state_transitions':[],
            'note':'Every solve runs within the fixed work of the manifest, so a point is compared once both versions complete it; a comparison is clean when their replay options, the work each level took among them, are the same. Off over 10 s counts the representatives whose optimized off command took longer. Diagnostic on, CI collection success and ordinary tests do not rescue an off timeout. Each point is one observation, not a repeat distribution.'}
    if set(summaries)=={'baseline','candidate'}:
        left={key(p):p for p in summaries['baseline']['pairs']};right={key(p):p for p in summaries['candidate']['pairs']}
        for item in sorted(left.keys()|right.keys()):
            a,c=left.get(item),right.get(item)
            kind=transition(a,c)
            if kind=='common_points':
                seconds_a,seconds_c=a['off_command_seconds_observed'],c['off_command_seconds_observed']
                clean=clean_comparison(a,c)
                result[kind].append({'point':item,'baseline_seconds_observed':seconds_a,'candidate_seconds_observed':seconds_c,
                    'clean_comparison':clean,'baseline_pressure_observed':pressure(a),'candidate_pressure_observed':pressure(c),
                    'difference_seconds':seconds_c-seconds_a if clean else None,'candidate_over_baseline':seconds_c/seconds_a if clean and seconds_a else None,
                    'off_output_bytes_equal':exact_outputs_equal(root,a,c),'off_file_and_record_hashes_equal':a['off_output_files']==c['off_output_files'],
                    'default_options_equal':a['off_options_canonical']==c['off_options_canonical'],
                    'profile_on_observed_work_equal':work(a)==work(c) if a['on_status']==c['on_status']=='COMPLETE' else None,
                    'profile_work_comparison_note':'Complete diagnostic-on metrics only, excluding times. Partial is unknown; never infer all off work from diagnostic on.'})
            else:
                family=item[1] if item[0]=='synthetic' else None
                result[kind].append({'point':item,'baseline_status':a['off_status'] if a else 'NOT_RUN',
                    'candidate_status':c['off_status'] if c else 'NOT_RUN',
                    'baseline_pressure_observed':pressure(a) if a else None,'candidate_pressure_observed':pressure(c) if c else None,
                    'baseline_stop':summaries['baseline']['stops'].get(family),'candidate_stop':summaries['candidate']['stops'].get(family),
                    'speed_ratio':None,'note':'NOT_RUN and ERROR are not measured timeout improvements. A timeout has no completed baseline time.'})
    b.save(root/'comparison.json',result)
    lines=['# Optimized solver measurements','',result['note'],'',f'| Source | Collection complete | Observed representative off / {b.REPRESENTATIVES} | Off over 10 s | Complete off/on output mismatches |','| --- | --- | ---: | ---: | ---: |']
    for label,summary in summaries.items():
        lines.append(f'| {label}: {summary["build"]["source_head"]} | {summary["collection_complete"]} | {summary["observed_representative_off_runs"]}/{b.REPRESENTATIVES} | {summary["representative_off_10s_failures"]} | {summary["complete_output_mismatches"]} |')
    labels=list(summaries)
    lines+=['','| Point | '+' | '.join(f'{label} off s' for label in labels)+' |','| --- |'+' ---: |'*len(labels)]
    observed={label:{key(p):p for p in summary['pairs']} for label,summary in summaries.items()}
    for point in sorted({k for pairs in observed.values() for k in pairs},key=str):
        cells=[]
        for label in labels:
            p=observed[label].get(point)
            seconds=p.get('off_command_seconds_observed') if p else None
            cells.append('NOT_RUN' if p is None else f'{seconds:.2f}' if p['off_status']=='COMPLETE' and isinstance(seconds,(int,float)) else p['off_status'])
        lines.append('| '+'/'.join(map(str,point[1:]))+' | '+' | '.join(cells)+' |')
    lines+=['',f'Common observed passing points: {len(result["common_points"])} (clean comparison: {sum(p["clean_comparison"] for p in result["common_points"])}). Measured TIMEOUT to complete: {len(result["newly_completed_off_points"])}. Candidate-only measured: {len(result["candidate_only_measured"])}. Observed timeout regressions: {len(result["regressed_off_points"])}.',
            '', 'Read raw measurement.json, output bytes, profile.json and resource/ownership records in each version directory. Partial/incomplete results provide no complete equality or precise observer overhead conclusion. Windows GNU CI is not a universal desktop/Web guarantee.']
    text='\n'.join(lines)+'\n';(root/'comparison.md').write_text(text,encoding='utf-8')
    if os.environ.get('GITHUB_STEP_SUMMARY'):
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a',encoding='utf-8') as stream:stream.write(text)
    print(text)


if __name__=='__main__':
    main()
