; Hooks into the NSIS installer that the Tauri bundler writes (tauri.conf.json, bundle > windows >
; nsis > installerHooks).

; Uninstalling leaves nothing behind. Tauri's uninstaller keeps the install location, under
; Software\<publisher>\<product>, unless "Delete the application data" is checked, which a silent
; uninstall cannot do; an update, which runs the uninstaller with /UPDATE, still keeps it for the
; installer that follows.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"
  ${EndIf}
!macroend
