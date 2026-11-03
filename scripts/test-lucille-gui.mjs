// Reproducible browser-level automation of the real, dedicated Tauri QA WebView.
// This bypasses only the failing Windows screenshot transport, not backend IPC,
// SQLite, executor approvals, or filesystem scope checks. No inference is used.
import { createRequire } from 'node:module'
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
const { chromium } = createRequire(import.meta.url)('@playwright/test')
const root=resolve('.lucille-qa-runtime/lucille-synthetic-gui')
if(readFileSync(join(root,'.lucille-isolated'),'utf8')!=='synthetic-data-only\n')throw new Error('Synthetic QA marker required')
const reports=resolve('.lucille-qa-runtime/reports');mkdirSync(reports,{recursive:true})
const browser=await chromium.connectOverCDP('http://127.0.0.1:15275')
const pages=browser.contexts().flatMap(context=>context.pages()).filter(page=>page.url()==='http://tauri.localhost/qa/lucille.html')
if(pages.length!==1)throw new Error('Expected exactly one dedicated QA WebView')
const page=pages[0], evidence=[]
const assert=(condition,message)=>{if(!condition)throw new Error(message)}
async function send(mode){
  await page.getByRole('radio',{name:mode.toUpperCase(),exact:true}).check()
  const response=page.waitForResponse(value=>value.url()==='http://127.0.0.1:15274/v1/chat/completions'&&value.request().method()==='POST')
  await page.getByRole('button',{name:'Send synthetic request',exact:true}).click()
  const body=await(await response).json();const text=body.choices[0].message.content
  await page.getByText(text,{exact:true}).waitFor()
  return {mode,text,messages:JSON.parse(await page.locator('pre').last().innerText())}
}
await page.getByRole('button',{name:'Refresh persisted state',exact:true}).click()
await page.waitForFunction(()=>!globalThis.document.querySelector('fieldset').disabled)
await page.getByRole('button',{name:'Initialize synthetic identity',exact:true}).click()
await page.getByText(/Applied profile: Synthetic Aurora/).waitFor()
await page.getByRole('checkbox',{name:'Explicit Lucille profile binding'}).check()
for(const mode of ['chat','agent','code']){
  const sent=await send(mode)
  assert(sent.text.includes('Synthetic Aurora')&&sent.text.includes('telescope is blue'),'Identity/memory missing in '+mode)
  assert(sent.messages.filter(message=>message.content.startsWith('User-configured persistent identity')).length===1,'Duplicate identity')
  assert(mode!=='code'||!sent.messages.some(message=>message.content.startsWith('You are the Coding Agent')),'Generic Code identity')
  evidence.push({mode,identity_and_memory:sent.text})
}
await page.screenshot({path:join(reports,'identity-code.png')})
await page.getByRole('button',{name:'Verify real file tools',exact:true}).click()
await page.getByText(/Real synthetic filesystem calls completed/).waitFor()
const receipts=JSON.parse(await page.locator('pre').nth(2).innerText())
assert(receipts.filter(receipt=>receipt.result_confirmed).length===4&&receipts.at(-1).outcome==='failed','Invalid real executor evidence')
evidence.push({filesystem:receipts})
await page.getByRole('button',{name:'Update synthetic memory',exact:true}).click()
await page.getByText(/active v\d+: The synthetic telescope is green\./).waitFor()
for(const mode of ['chat','agent','code']){
  const sent=await send(mode);assert(sent.text.includes('telescope is green')&&!sent.text.includes('telescope is blue'),'Stale updated memory');evidence.push({mode,updated:sent.text})
}
await page.getByRole('button',{name:'Delete synthetic memory',exact:true}).click()
await page.getByText('No records',{exact:true}).waitFor()
for(const mode of ['chat','agent','code']){
  const sent=await send(mode);assert(sent.text.includes('memories=none'),'Deleted memory leaked');evidence.push({mode,deleted:sent.text})
}
await page.getByRole('checkbox',{name:'Explicit Lucille profile binding'}).uncheck()
const native=await send('code');assert(native.text.includes('Coding Agent (default)'),'Native Code behavior changed')
evidence.push({disabled:native.text})
await page.getByRole('checkbox',{name:'Explicit Lucille profile binding'}).check()
await page.screenshot({path:join(reports,'deletion-and-disable.png')})
writeFileSync(join(reports,'gui-results.json'),JSON.stringify({captured_at:new Date().toISOString(),transport:'dedicated-WebView-CDP',real_inference:false,evidence},null,2))
console.log('PASS: dedicated GUI identity/memory in all three modes, update/delete, native fallback and real scoped file tools.')
// Detach without closing the user's QA application.
await browser.close()
