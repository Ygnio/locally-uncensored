$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$real=(Resolve-Path (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-real21c')).Path
if([IO.File]::ReadAllText((Join-Path $real '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Synthetic marker required'}
$session=Get-Content -LiteralPath (Join-Path $real 'gui-processes.json') -Raw | ConvertFrom-Json
if($session.real_root -ne $real){throw 'Wrong manifest root'}
foreach($record in @($session.gui,$session.vite)){
 if(-not $record){continue}
 $p=Get-CimInstance Win32_Process -Filter "ProcessId=$($record.pid)"
 if(-not $p){continue}
 if($p.ExecutablePath -ne $record.executable -or $p.CreationDate -ne [datetime]$record.created -or $p.CommandLine -ne $record.command){throw 'Own PID/path/creation/command identity changed; no stop allowed'}
 Stop-Process -Id $p.ProcessId
}
Write-Output 'Closed only manifest-verified own GUI and Vite; no model or existing app stopped.'
