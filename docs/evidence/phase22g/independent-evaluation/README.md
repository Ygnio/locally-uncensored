# Avaliação independente de memória da Lucille · 2.2G

## Desenho registrado

A pergunta da avaliação é se a memória sustenta uma resposta útil, fiel e completa numa conversa nova, inclusive depois que o processo Tauri termina. O produto sob avaliação é o commit `97c3f31a187bd10e40b74122f15c40fe7500c2be`. O executor dedicado é `614a736260f1b5f294897646011fa72fd801e50f`. O commit que contém este documento e o manifesto é a âncora do congelamento.

O arquivo `inputs.json` fixa a ordem de 28 mensagens de preparação e 40 consultas. Cada uma das cinco identidades fictícias recebe um perfil WebView e um banco novo. Há cinco consultas para cada uma das oito capacidades, 17 em Chat, 13 em Code e 10 em Agent. Cada mensagem abre uma conversa vazia. As cinco trocas de identidade exigem cinco inicializações iniciais no total. Mais dez etapas exigem encerramento e reabertura do mesmo perfil, com mudança comprovada de processo. Cinco reapresentações usam a chave exata da interação original já excluída. O replay nativo ocorre antes da consulta, em ação separada do driver.

| Identidade | Projeto principal | Relação que exige distinguir entidades |
|---|---|---|
| Nira Socal | Torvelume | Finalidade das travessias, numeração dos bilhetes e escolha de banco |
| Ênio Trancil | Cimbrocha | Bancada e equipamento de campo, captura real e experiência planejada |
| Ilda Rovaz | Marulheta | Tema pessoal, texto de embalagem, finalidade do laudo e dados recebidos |
| Lino Peiral | Pampiluz | Preferência de revisão, entrevista, especificação e hipótese de patrocínio |
| Salma Bravim | Lousamar | Torre, notebook antes e depois da oficina, tarefa executada e próxima tarefa |

Os perfis alternam projeto ou escopo, inclusive com o mesmo nome de projeto em escopos distintos. A leitura do gabarito começa pelo ID da consulta em `oracle.json`. `original_sources` aponta para os turnos em `inputs.json`. O catálogo `facts` identifica titular, propriedade, valor e modalidade. Os mapas `initial` e `expected` cobrem todo o perfil, com estados atual, histórico e excluído. Chaves omitidas ainda não foram materializadas. `absent` e `forbidden_claims` especificam contraprovas. Os IDs semânticos do gabarito precisam de correspondência demonstrada com as linhas e versões nativas. Nenhum ID semântico vai ao modelo. A política de revisão é a mesma do driver anterior, com revisão via interface dos candidatos e conflitos elegíveis após as fontes, inclusive preferências. Uma revisão nunca autoriza antecipar a confirmação ao usuário.

## Execução posterior no checkout Windows

Identificar no histórico do PR o commit específico **Freeze independent memory evaluation before inference**. Usar seu SHA completo nos comandos. Conferir a revisão do produto e do executor no manifesto. Na máquina preparada, em PowerShell na raiz do checkout, executar:

```powershell
$freeze = (git log -1 --format=%H --fixed-strings --grep='Freeze independent memory evaluation before inference').Trim()
python scripts/lucille-independent.py check --freeze $freeze
python qa/independent_evaluation_test.py
```

O comando `check` verifica os bytes contra o manifesto ancorado no Git, as regressões, as cinco portas livres, a ausência de perfis de campanha e a integridade dos 104 arquivos protegidos. Confere os pesos, o servidor, o executável nativo, o loader e a árvore do frontend contra `runtime-lock.json`. Os artefatos de execução são locais. Em outra máquina, provisionar a infraestrutura de QA com os mesmos pesos e binários e confirmar as impressões digitais antes de expor casos. O comando opcional `python scripts/lucille-independent.py smoke` inspeciona o perfil exclusivo de ensaio duas vezes com o broker e os servidores desligados.

Somente na campanha futura autorizada, executar:

```powershell
python scripts/lucille-independent.py run --freeze $freeze
```

O controlador reserva o diretório `.lucille-qa-runtime/lucille-synthetic-real22f/independent-v1` exclusivamente uma vez e usa a trava do controlador existente. Inicia apenas os serviços próprios em loopback, com Nomic CPU em 15289, Gemma 4 12B em 15288 e broker em 15284. O frontend real usa a porta 15283 e reserva 15287 para inspeção. O contexto operacional do Gemma é 65.536 tokens. As pastas de aplicação, armazenamento WebView, usuário e temporários apontam para o perfil isolado. O driver usa a composição de mensagens da aplicação e a API nativa Tauri. Os modos Code e Agent usam a área padrão do chat, sem abrir diretório de trabalho do repositório, fornecer arquivos de fixture ou escolher pastas. Qualquer pedido de aprovação de ferramenta externa interrompe a etapa. O avaliador mantém o gabarito fora da GUI.

O transportador aceita texto, modo e vinculação previstos. Cada consulta abre uma conversa nova. O contexto persistente recuperado pode revelar a resposta. Ao revisar o primeiro pedido bruto, confirmar uma única pergunta efetiva, ausência de diálogo anterior e origem autorizada de cada trecho recuperado. Verificar a correspondência de ID, versão, escopo, projeto e estado no SQLite. Um vazamento invalida a medição e interrompe a campanha. Se não houve geração, consultar a trilha de transporte, DOM e retenção para identificar o resultado.

Após cada etapa, um avaliador consulta o gabarito e os arquivos locais `*-before.json`, `*-observed.json`, `*-broker.ndjson` e, quando existir, `*-replay.json`. O driver captura revisões, cronologia de DOM, tentativas, mensagens entregues, jobs e recibos de operações. O supervisor captura RAM, VRAM e processos em `reports`. Registrar no comentário do julgamento as referências concretas consultadas. Declarar ausência de comprovação se a ordem de dois eventos não puder ser estabelecida. Em outro terminal, substituir o exemplo pelo ID em espera e pelo juízo apurado:

```powershell
python scripts/lucille-independent.py adjudicate i1-01 ok "Indicar arquivo, recibo, observação e fundamento"
```

As alternativas são `ok`, `functional` e `critical`. Uma falha funcional comum recebe registro e a sequência prossegue sem correção. Usar `critical` para entrega crítica, confirmação prematura, reintrodução indevida ou risco de isolamento. Falhas de infraestrutura interrompem o controlador. Conservar o perfil, os relatórios do driver inclusive os parciais, as amostras de recursos e a lista de tickets. O arquivo `closure.json` distingue pendências, interrupções e observações. Examinar também `interruption.json` e eventuais erros de encerramento. O controlador verifica hashes e portas ao finalizar, inclusive após exceção. Revalidar a âncora offline após o encerramento usando `frozen` e `integrity` do executor. O comando completo `check` exige ambiente virgem.

## Orçamento e parada

Planejar 68 gerações iniciais e até 68 extrações, pois até uma consulta pode produzir trabalho de aprendizagem. Reservar o restante para reparos do verificador e passos adicionais do Agent. O teto agregado de **160 tentativas Gemma** inclui falhas, extrações e regenerações. A repartição de planejamento é 68 + 68 + 14 reparos + 10 passos adicionais. No cenário de uma extração por fonte e 10–20 passos adicionais, estimam-se 106–116 tentativas. Registrar tokens de entrada e saída retornados por chamada, tickets sem resposta e latência. O frontend solicita até 384 tokens de saída e o broker limita cada chamada a 768. Estimativa de planejamento de 100–220 mil tokens totais, sensível ao tamanho dos contextos e às chamadas internas.

Prever cerca de 100–200 chamadas de embeddings entre consultas, fontes e indexação. Contar cada pedido Nomic efetivo à parte. Há 15 inicializações do host e uma carga inicial de cada modelo. Estimar 30–90 minutos de máquina mais 1–3 horas para revisão humana. O checkpoint temporal anterior registrou mínimo de cerca de 2,0 GiB de RAM física livre e pico de 9.427 MiB de VRAM usada no dispositivo. São observações daquela máquina. Durante a campanha, registrar mínimo de RAM física e commit disponível, pico de VRAM, amostras GPU, latências de geração/extração e duração incluindo revisões.

Parar ao atingir o orçamento para nova tentativa, abaixo de 1 GiB físico livre, sob pressão de commit, perda de heartbeat/porta/processo, integridade SQLite ou chave estrangeira inválida, violação de escopo ou ameaça a dados operacionais. Preservar a reserva consumida por uma tentativa que falhou. Um erro de infraestrutura no meio de uma consulta deixa a consulta interrompida e as seguintes pendentes. Registrar separadamente qualquer falha funcional já observável na etapa interrompida. Uma entrega crítica exige retenção da evidência e parada antes da próxima entrada. Proibido reaproveitar o diretório para reiniciar a campanha ou apagar tickets para continuar.

## Proveniência e limites

A revisão textual comparou as consultas com 14.065 fragmentos interrogativos históricos. O maior Jaccard de trigramas observado foi 0,111. O arquivo `novelty.json` discrimina a comparação por caso. A revisão de cenários examinou ainda autoria, troca de entidade, finalidade e modalidade. Frases curtas de preparação, sobretudo diretivas de esquecimento, podem exigir vocabulário comum. A autoria e a revisão ocorreram nesta preparação, sem adjudicação externa cega. A amostra compartilha cinco histórias, uma implementação, idioma e configuração. Os resultados só descreverão estas consultas e estas condições.

Para qualquer revisão antes da primeira exposição, registrar justificativa, versão, novo manifesto e novo commit. Uma vez exposto um caso, preservar seus resultados e o gabarito original. Se o produto precisar de correção, promover os casos expostos a regressão e preparar novos casos para outra avaliação independente. Nunca substituir um caso difícil na amostra em curso.

`regressions.json` fixa evidências e testes conhecidos. Inclui C1 (21/21 obrigatórios, 26/26 total), C2 (27/27), admissão (20/20), decisão temporal, Alfenim após reinício, Vimeira e autoria, tombstone com replay e a matriz de 19 aceitações corretas e 17 bloqueios corretos. Para a regressão temporal isolada usar `npx vitest run src/lib/__tests__/lucille-temporal-claims.test.ts`. As entradas C1/C2 e o executor de recuperação encontram-se no conjunto congelado 2.2F e nos caminhos do índice. A preparação verificou a disponibilidade e os hashes dos artefatos. Os números históricos não contam como resultados das 40 consultas.
