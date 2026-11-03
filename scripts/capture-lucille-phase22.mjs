import { createRequire } from 'node:module'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const label=process.argv[2];assert(/^[a-z0-9-]{1,60}$/.test(label))
const root=resolve('.lucille-qa-runtime/lucille-synthetic-real22');assert.equal(readFileSync(join(root,'.lucille-isolated'),'utf8'),'synthetic-data-only\n')
const file=join(root,'reports',label+'.png');assert(!existsSync(file),'Preserve prior screenshot')
const {chromium}=createRequire(import.meta.url)('@playwright/test');const browser=await chromium.connectOverCDP('http://127.0.0.1:15287')
try{const page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/');assert(page);assert(await page.evaluate(()=>Boolean(globalThis.__TAURI_INTERNALS__)),'Actual native WebView required');await page.screenshot({path:file,fullPage:true});writeFileSync(join(root,'reports',label+'-capture.json'),JSON.stringify({captured:new Date().toISOString(),browser:browser.version(),url:page.url(),native:true},null,2));console.log(file)}finally{await browser.close()}
