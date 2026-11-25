; Windows installer hooks (#505, design D4). Merged into the build by
; installer.windows.conf.json, which the Windows release job passes to
; `tauri build --config`. The installer is per-user (no administrator), so the
; binaries go to $INSTDIR\bin and onto the USER Path.
;
; Path is edited through PowerShell's [Environment] API rather than NSIS string
; functions: NSIS strings are capped at 1024 characters, and a longer Path read
; through them comes back truncated and would be written back that way. The API
; also broadcasts the change, so new terminals see it.

!macro SENSEI_STOP_DAEMON
  ; A running senseid.exe holds its file open; stop it (and its logon task)
  ; before files are replaced or removed. A failure here is not fatal: on a
  ; first install there is nothing to stop.
  IfFileExists "$INSTDIR\bin\sensei.exe" 0 +2
    nsExec::ExecToLog '"$INSTDIR\bin\sensei.exe" stop'
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro SENSEI_STOP_DAEMON
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d = '$INSTDIR\bin'; $$p = [Environment]::GetEnvironmentVariable('Path', 'User'); $$parts = @(); if ($$p) { $$parts = $$p -split ';' | Where-Object { $$_ } }; if ($$parts -notcontains $$d) { [Environment]::SetEnvironmentVariable('Path', (($$parts + $$d) -join ';'), 'User') }"`
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro SENSEI_STOP_DAEMON
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d = '$INSTDIR\bin'; $$p = [Environment]::GetEnvironmentVariable('Path', 'User'); if ($$p) { $$parts = $$p -split ';' | Where-Object { $$_ -and $$_ -ne $$d }; [Environment]::SetEnvironmentVariable('Path', ($$parts -join ';'), 'User') }"`
!macroend
