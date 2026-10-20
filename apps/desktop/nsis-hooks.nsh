; Hooks into the NSIS installer that the Tauri bundler writes (tauri.conf.json, bundle > windows >
; nsis > installerHooks).

; Register a candidate in modern Windows' Open With and Default Apps interfaces.
; Leave extension defaults and UserChoice intact, including during updates/uninstall.
!macro RNL_REGISTER_EXTENSION EXT
  WriteRegStr SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "RNLViewer.Netlist" ""
  WriteRegStr SHCTX "Software\Rich Netlist\RNL Viewer\Capabilities\FileAssociations" ".${EXT}" "RNLViewer.Netlist"
!macroend

!macro RNL_UNREGISTER_EXTENSION EXT
  DeleteRegValue SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "RNLViewer.Netlist"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.${EXT}\OpenWithProgids"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\RNLViewer.Netlist" "" "Netlist or RNL package"
  WriteRegStr SHCTX "Software\Classes\RNLViewer.Netlist\DefaultIcon" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0'
  WriteRegStr SHCTX "Software\Classes\RNLViewer.Netlist\shell\open\command" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"'
  WriteRegStr SHCTX "Software\Classes\RNLViewer.Netlist\Application" "ApplicationName" "RNL Viewer"
  WriteRegStr SHCTX "Software\Rich Netlist\RNL Viewer\Capabilities" "ApplicationName" "RNL Viewer"
  WriteRegStr SHCTX "Software\Rich Netlist\RNL Viewer\Capabilities" "ApplicationDescription" "View and edit SPICE/Spectre netlists and RNL packages."
  WriteRegStr SHCTX "Software\Rich Netlist\RNL Viewer\Capabilities" "ApplicationIcon" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0'
  WriteRegStr SHCTX "Software\RegisteredApplications" "RNL Viewer" "Software\Rich Netlist\RNL Viewer\Capabilities"
  !insertmacro RNL_REGISTER_EXTENSION "sp"
  !insertmacro RNL_REGISTER_EXTENSION "spi"
  !insertmacro RNL_REGISTER_EXTENSION "spice"
  !insertmacro RNL_REGISTER_EXTENSION "cir"
  !insertmacro RNL_REGISTER_EXTENSION "ckt"
  !insertmacro RNL_REGISTER_EXTENSION "net"
  !insertmacro RNL_REGISTER_EXTENSION "netlist"
  !insertmacro RNL_REGISTER_EXTENSION "inc"
  !insertmacro RNL_REGISTER_EXTENSION "lib"
  !insertmacro RNL_REGISTER_EXTENSION "scs"
  !insertmacro RNL_REGISTER_EXTENSION "rnl"
  !insertmacro UPDATEFILEASSOC
!macroend

; Uninstalling leaves nothing behind. Tauri's uninstaller keeps the install location, under
; Software\<publisher>\<product>, unless "Delete the application data" is checked, which a silent
; uninstall cannot do; an update, which runs the uninstaller with /UPDATE, still keeps it for the
; installer that follows.
!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro RNL_UNREGISTER_EXTENSION "sp"
  !insertmacro RNL_UNREGISTER_EXTENSION "spi"
  !insertmacro RNL_UNREGISTER_EXTENSION "spice"
  !insertmacro RNL_UNREGISTER_EXTENSION "cir"
  !insertmacro RNL_UNREGISTER_EXTENSION "ckt"
  !insertmacro RNL_UNREGISTER_EXTENSION "net"
  !insertmacro RNL_UNREGISTER_EXTENSION "netlist"
  !insertmacro RNL_UNREGISTER_EXTENSION "inc"
  !insertmacro RNL_UNREGISTER_EXTENSION "lib"
  !insertmacro RNL_UNREGISTER_EXTENSION "scs"
  !insertmacro RNL_UNREGISTER_EXTENSION "rnl"
  DeleteRegValue SHCTX "Software\RegisteredApplications" "RNL Viewer"
  DeleteRegKey SHCTX "Software\Classes\RNLViewer.Netlist"
  DeleteRegKey SHCTX "Software\Rich Netlist\RNL Viewer\Capabilities"
  !insertmacro UPDATEFILEASSOC
  ${If} $UpdateMode <> 1
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"
  ${EndIf}
!macroend
