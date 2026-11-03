// Only the debug validation host loads this. Never imports any prior profile.
if (!globalThis.localStorage.getItem('lucille-phase16-seeded')) {
  // Version zero intentionally runs the real additive settings migration,
  // filling every application default before applying these fixture choices.
  globalThis.localStorage.setItem('chat-settings', JSON.stringify({version:0,state:{settings:{onboardingDone:true,appMode:'local',cloudTeasersEnabled:false,codexDefaultMode:'ask',agentMaxIterations:8,language:'en',defaultWorkspace:{kind:'sandbox'}}}}))
  globalThis.localStorage.setItem('lu-providers', JSON.stringify({version:2,state:{providers:{openai:{id:'openai',name:'Synthetic payload inspector',enabled:true,baseUrl:'http://127.0.0.1:15284/v1',apiKey:'',isLocal:true,managed:false},ollama:{id:'ollama',enabled:false},anthropic:{id:'anthropic',enabled:false},'lu-cloud':{id:'lu-cloud',enabled:false}},openaiEntries:[]}}))
  globalThis.localStorage.setItem('chat-models',JSON.stringify({state:{activeModel:'openai::aurora-qa',lastLocalModel:'openai::aurora-qa',autoCloudPickCleared:true}}))
  globalThis.localStorage.setItem('lucille-phase16-seeded','synthetic-only')
}
