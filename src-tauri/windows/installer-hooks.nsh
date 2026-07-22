; Stops an orphaned Wall mpv from versions that predate Job Object ownership.
!define WALL_HOOK_DIR "${__FILEDIR__}"
!define WALL_POWERSHELL "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
!define WALL_CLEANUP_SCRIPT "$PLUGINSDIR\wall-cleanup-mpv.ps1"

!macro NSIS_HOOK_PREINSTALL
    InitPluginsDir
    File /oname=${WALL_CLEANUP_SCRIPT} "${WALL_HOOK_DIR}\cleanup-mpv.ps1"
    nsExec::ExecToStack \
        '"${WALL_POWERSHELL}" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass \
        -File "${WALL_CLEANUP_SCRIPT}" -TargetPath "$INSTDIR\mpv.exe"'
    Pop $0
    Pop $1
    StrCmp $0 "0" wall_cleanup_mpv_done
    MessageBox MB_ICONSTOP|MB_OK "无法结束旧版 Wall 播放器：$1"
    Abort
wall_cleanup_mpv_done:
!macroend
