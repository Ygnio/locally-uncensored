param([Parameter(Mandatory=$true)][string]$Root)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$qaRoot=(Resolve-Path -LiteralPath $Root).Path
if($qaRoot -ne (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-real22') -or [IO.File]::ReadAllText((Join-Path $qaRoot '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Exact marked Phase22 root required'}
$stop=Join-Path $qaRoot 'sampler.stop'
$log=Join-Path $qaRoot 'reports\system-resources-final.ndjson'
if(Test-Path -LiteralPath $log){throw 'Never overwrite final samples'}
while(-not(Test-Path -LiteralPath $stop)){
 $mem=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory
 $page=Get-CimInstance Win32_PageFileUsage | Select-Object AllocatedBaseSize,CurrentUsage,PeakUsage
 $gui=$null;$manifest=Join-Path $qaRoot 'gui-processes.json'
 if(Test-Path -LiteralPath $manifest){
  $record=(Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json).gui
  $actual=Get-CimInstance Win32_Process -Filter "ProcessId=$($record.pid)"
  if($actual -and $actual.ExecutablePath -eq $record.executable -and $actual.CreationDate -eq [datetime]$record.created){$p=Get-Process -Id $record.pid;$gui=@{working_set=$p.WorkingSet64;private_bytes=$p.PrivateMemorySize64}}
 }
 $db=Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-full22\lucille-phase1.sqlite3'
 $files=@{}
 foreach($path in @($db,($db+'-wal'),($db+'-shm'))){if(Test-Path -LiteralPath $path){$files[[IO.Path]::GetFileName($path)]=(Get-Item -LiteralPath $path).Length}}
 @{utc=[DateTime]::UtcNow.ToString('o');available_mib=$mem.AvailableMBytes;committed_bytes=$mem.CommittedBytes;commit_limit=$mem.CommitLimit;pages_per_second=$mem.PagesPersec;pagefile=$page;own_gui=$gui;own_sqlite_files=$files} | ConvertTo-Json -Depth 6 -Compress | Add-Content -LiteralPath $log -Encoding utf8
 Start-Sleep -Seconds 10
}
