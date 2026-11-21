; Gerado por scripts/i18n.mjs sync — não edite à mão.
; Grava o idioma escolhido no instalador; o Sentinel usa na primeira vez que abre.
!include "LogicLib.nsh"

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $0 "pt-BR"
  ${If} $LANGUAGE == 1046
    StrCpy $0 "pt-BR"
  ${ElseIf} $LANGUAGE == 1033
    StrCpy $0 "en"
  ${EndIf}
  WriteRegStr HKCU "Software\Sentinel" "Language" $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Sentinel"
!macroend
