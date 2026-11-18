param([Parameter(Mandatory=$true)][string]$Root)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$qaRoot=(Resolve-Path -LiteralPath $Root).Path
if($qaRoot -ne (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-real22f') -or [IO.File]::ReadAllText((Join-Path $qaRoot '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Exact marked Phase 2.2F root required'}
$stop=Join-Path $qaRoot 'sampler.stop'
$log=Join-Path $qaRoot 'reports\system-resources-22f.ndjson'
if(Test-Path -LiteralPath $log){throw 'Existing resource evidence must be kept under its own filename'}
$nvidia=Get-Command 'nvidia-smi.exe' -ErrorAction SilentlyContinue

function Measure-Owned([int]$ProcessId){
 $process=Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
 if($process){return @{working_set_bytes=$process.WorkingSet64;private_bytes=$process.PrivateMemorySize64;cpu_seconds=$process.CPU}}
}
function Measure-ManifestModel([string]$Role){
 $manifest=Join-Path $qaRoot "$Role-process.json"
 if(-not (Test-Path -LiteralPath $manifest)){return}
 try {
  $record=Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
  if($record.PSObject.Properties.Name -contains 'exit_code'){return}
  $actual=Get-Process -Id $record.pid -ErrorAction SilentlyContinue
  if($actual -and $actual.Path -eq $record.executable -and $actual.StartTime.ToFileTimeUtc() -eq [long]$record.creation_filetime){return Measure-Owned $record.pid}
 }catch{ return }
}
function Measure-OwnedGui{
 $manifest=Join-Path $qaRoot 'gui-processes.json'
 if(-not (Test-Path -LiteralPath $manifest)){return}
 try {
  $owner=(Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json).gui
  if(-not $owner){return}
  $actual=Get-CimInstance Win32_Process -Filter "ProcessId=$([int]$owner.pid)"
  if(-not $actual -or $actual.ExecutablePath -ne $owner.executable -or $actual.CreationDate -ne [datetime]$owner.created){return}
  $webviews=@(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object CreationDate -ge $actual.CreationDate)
  $lineage=[Collections.Generic.HashSet[int]]::new()
  $lineage.Add([int]$owner.pid) | Out-Null
  $attached=@()
  do {
   $joined=@($webviews | Where-Object {$lineage.Contains([int]$_.ParentProcessId) -and -not $lineage.Contains([int]$_.ProcessId)})
   foreach($child in $joined){
    $lineage.Add([int]$child.ProcessId) | Out-Null
    $sample=Measure-Owned $child.ProcessId
    if($sample){$attached+=$sample}
   }
  }while($joined.Count)
  return @{native=(Measure-Owned $owner.pid);webview_count=$attached.Count;webview_working_set_bytes=[long](($attached | ForEach-Object working_set_bytes | Measure-Object -Sum).Sum);webview_private_bytes=[long](($attached | ForEach-Object private_bytes | Measure-Object -Sum).Sum)}
 }catch{ return }
}

while(-not (Test-Path -LiteralPath $stop)){
 $memory=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory
 $cpu=(Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime
 $pagefile=Get-CimInstance Win32_PageFileUsage | Select-Object AllocatedBaseSize,CurrentUsage,PeakUsage
 $gpu=if($nvidia){@(& $nvidia.Source --query-gpu=index,memory.used,memory.total,utilization.gpu --format=csv,noheader,nounits 2>$null)}
 @{utc=[DateTime]::UtcNow.ToString('o');cpu_total_percent=$cpu;available_mib=$memory.AvailableMBytes;committed_bytes=$memory.CommittedBytes;commit_limit=$memory.CommitLimit;pages_per_second=$memory.PagesPersec;pagefile=$pagefile;gpu_csv_mib=$gpu;gui=(Measure-OwnedGui);gemma=(Measure-ManifestModel 'gemma');nomic=(Measure-ManifestModel 'nomic');broker=(Measure-ManifestModel 'broker')} | ConvertTo-Json -Depth 6 -Compress | Add-Content -LiteralPath $log -Encoding utf8
 Start-Sleep -Seconds 10
}
