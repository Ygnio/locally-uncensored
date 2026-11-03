param([string]$ContractPath)
# Read-only inventory; NEVER launches a model, sends inference, changes ports or reads chats.
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root=(Resolve-Path (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-full')).Path
if([IO.File]::ReadAllText((Join-Path $root '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Marked isolated root required'}
$os=Get-CimInstance Win32_OperatingSystem
$ports=@(Get-NetTCPConnection -LocalPort 8080,8081,8128,8188 -State Listen -ErrorAction SilentlyContinue | Select-Object LocalAddress,LocalPort,OwningProcess)
$models=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -match '^(llama-server|llama-cli|ollama|koboldcpp)\.exe$'} | Select-Object ProcessId,Name)
$gpu=$null
if(Get-Command nvidia-smi -ErrorAction SilentlyContinue){$gpu=@(nvidia-smi --query-gpu=name,memory.total,memory.used,memory.free --format=csv,noheader,nounits)}
$contractState='not supplied; real stage pending'
if($ContractPath){
 $resolved=(Resolve-Path -LiteralPath $ContractPath).Path
 if(-not $resolved.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Contract must be inside the isolated root'}
 $contract=Get-Content -LiteralPath $resolved -Raw | ConvertFrom-Json
 foreach($entry in @($contract.gemma,$contract.nomic)){
  if(-not $entry){throw 'Both model contracts required'}
  $endpoint=[uri]$entry.endpoint
  if($endpoint.Scheme -ne 'http' -or $endpoint.Host -ne '127.0.0.1' -or $endpoint.Port -notin @(15288,15289) -or $endpoint.UserInfo){throw 'Only exclusive loopback contract ports allowed'}
  if($entry.weights_sha256 -notmatch '^[a-fA-F0-9]{64}$' -or -not $entry.revision){throw 'Independent weight fingerprint and revision required'}
 }
 if($contract.gemma.context_tokens -ne 65536){throw 'Gemma context must remain 65536'}
 if($contract.nomic.model -ne 'nomic-ai/nomic-embed-text-v1.5' -or $contract.nomic.pooling -ne 'mean' -or $contract.nomic.document_prefix -ne 'search_document: ' -or $contract.nomic.query_prefix -ne 'search_query: '){throw 'Nomic contract mismatch'}
 $contractState='contract shape checked ONLY; server identity, weight hash, auth and resource use still require independent verification'
}
$result=@{observed_at=[DateTime]::UtcNow.ToString('o');mode='READ ONLY';free_ram_kib=$os.FreePhysicalMemory;total_ram_kib=$os.TotalVisibleMemorySize;gpu_csv_mib=$gpu;operational_listeners=$ports;model_process_names=$models;contract=$contractState;real_gemma_verified=$false;real_nomic_verified=$false;inference_sent=$false;model_started=$false;context_modified=$false;resource_estimate='File size is not total model/KV-cache/VRAM cost; 65536-token allocation requires verified architecture/cache configuration. No fit assumption.'}
$path=Join-Path $root 'reports\phase21b-preflight.json'
$result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $path
Write-Output 'Read-only preflight saved; no model started, no inference sent. Real stage remains pending.'
