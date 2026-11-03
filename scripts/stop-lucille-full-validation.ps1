$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root=(Resolve-Path (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-full')).Path
$session=Get-Content (Join-Path $root 'session.json') | ConvertFrom-Json
if($session.root -ne $root){throw 'Synthetic session root mismatch'}
$gui=Get-CimInstance Win32_Process -Filter "ProcessId=$($session.gui_pid)"
if($gui){
  if($gui.ExecutablePath -ne $session.executable -or -not $gui.ExecutablePath.EndsWith('\lucille-full-validation.exe')){throw 'GUI process identity mismatch'}
  Stop-Process -Id $gui.ProcessId
}
foreach($item in @(@{id=$session.vite_pid;pattern='vite.lucille-validation.config.ts'},@{id=$session.mock_pid;pattern='lucille-full-server.mjs'})){
  $process=Get-CimInstance Win32_Process -Filter "ProcessId=$($item.id)"
  if($process){
    if(-not $process.CommandLine.Contains($item.pattern)){throw 'Development process identity mismatch'}
    Stop-Process -Id $process.ProcessId
  }
}
Write-Output 'Closed only the manifest-identified full-validation GUI, frontend and mock. Synthetic data preserved.'
