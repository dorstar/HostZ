; NSIS hook: delete HostZ user data on uninstall
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState == 1
    SetShellVarContext current
    RmDir /r "$APPDATA\HostZ"
  ${EndIf}
!macroend
