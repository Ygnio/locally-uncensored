# Segunda tentativa da avaliação independente

Esta preparação registra o executor `e690687c453b1d39cacca67063eb1d784e5db9a4`. O produto continua em `97c3f31a187bd10e40b74122f15c40fe7500c2be`; entradas, gabarito, regras e manifesto continuam na âncora original `8b09ba6647961e9dee731010795b446add1ea2fc`. A primeira tentativa permanece arquivada em `independent-v1`, com relatório em `../independent-run-01`. O controlador confere o arquivo local de interrupção e os hashes dos logs retidos contra esse registro.

O supervisor agora importa `os` explicitamente antes de consultar as variáveis de ambiente. A inspeção de nomes globais nos sete módulos Python do caminho de inicialização e do teste não encontrou outra referência sem definição. Os cinco testes determinísticos passaram.

O teste de integração `python qa/independent_startup_test.py` usa um diretório sintético exclusivo, o lançador existente, as configurações reais de pressão de memória e os pesos fixados. Com ambos os modelos carregados simultaneamente, observou a primeira amostra de cada supervisor e consultou somente `GET /health`. O Nomic em CPU retornou 200 e ficou pronto em 6,36 s. O Gemma 4 12B com contexto de 65.536 retornou 200 e ficou pronto em 6,91 s. Os supervisores confirmaram a identidade dos dois processos no encerramento. As cinco portas ficaram livres, os 104 hashes protegidos conferiram e os logs do broker mantiveram seus tamanhos. Zero mensagens de avaliação, inferências Gemma ou embeddings. [Amostras e recibos](startup-test.json)

## Âncora e diretório

O commit de registro com o assunto abaixo vincula esta versão do executor à tentativa 02. O argumento `--freeze` recebe o SHA completo **deste registro**; internamente, o executor verifica o manifesto dos casos contra a âncora original, limita as diferenças entre versões aos arquivos de QA e documentação previstos e verifica a integridade do checkout.

```powershell
$registro = (git log -1 --format=%H --fixed-strings --grep='Register independent attempt 02 readiness').Trim()
python scripts/lucille-independent.py check --freeze $registro
```

A campanha futura usa o mesmo comando de execução do protocolo, com `run --freeze $registro`, adjudicação por etapa e teto de 160 tentativas. O controlador criará exclusivamente `.lucille-qa-runtime/lucille-synthetic-real22f/independent-v2` ao iniciar essa campanha. Os cinco perfis, os tickets e os resultados devem estar ausentes no pré-teste. As capturas do driver usarão o prefixo `g22ie2-`. Esta preparação termina antes da criação do diretório da campanha.
