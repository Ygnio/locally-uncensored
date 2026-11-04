# Avaliação isolada da Fase 2.2

Executar a partir da raiz do checkout, com os executáveis nativos próprios já construídos e os modelos desligados durante builds. Os helpers exigem os diretórios marcados `.lucille-qa-runtime/lucille-synthetic-real22` e `lucille-synthetic-full22`, contrato privado atestado, pesos existentes, portas próprias livres e chaves locais. O contrato público de evidência possui caminhos removidos e não é uma configuração executável.

O manifesto congelado registra os hashes dos binários efetivamente usados. Outro build ou outro computador exige uma nova atestação; não editar um manifesto histórico para fazer um executável diferente parecer o original. Os conjuntos desta fase, agora públicos, passam a servir de regressão. Uma nova alegação de qualidade independente exige um corpus novo, congelado após o desenvolvimento.

## Comandos usados

```powershell
python scripts/run-lucille-phase22-owned.py --root .lucille-qa-runtime/lucille-synthetic-real22 --operation start
python scripts/test-lucille-phase22-extraction.py --root .lucille-qa-runtime/lucille-synthetic-real22 --bridge ../build/memory/debug/examples/real_validation_bridge.exe --dataset crates/lucille-memory/tests/fixtures/phase22/extraction-v3.json --label final-v3 --freeze crates/lucille-memory/tests/fixtures/phase22/freeze-v3.json
```

O avaliador original interrompeu no caso V3-035. `resume-phase22-v3-extraction.py` conserva a resposta real já produzida nesse caso, registra recusas do bridge e consulta o estado canônico sem inventar claims ou reparar respostas. Outros erros de processo continuam encerrando o ensaio. Consultar `v3-measurement-recovery.json` e `v3-bridge-rejections.ndjson` nas evidências. O helper serve para reproduzir essa recuperação específica, não como um mecanismo genérico de ignorar erros.

`run-phase22-v3-series.py` aguarda a extração completa, executa a recuperação com Nomic em banco separado, abre o host Tauri e envia os planos de 20/50/100/200 turnos. `run-phase22-v3-lifecycle.py` aguarda a série longa e então reinicia somente o host próprio e o Nomic próprio. Todas as conversas usam o compositor e os controles da aplicação; os probes criam novas sessões. Os controles transitórios ficam em `reports/control.json`, e resultados finais não são sobrescritos.

```powershell
python qa/phase22-evaluation/run-phase22-v3-series.py
python qa/phase22-evaluation/run-phase22-v3-lifecycle.py
node qa/phase22-evaluation/phase22-v3-fault.mjs cancel 1
node qa/phase22-evaluation/phase22-v3-fault.mjs restart 1
python scripts/test-lucille-phase22-context.py --root .lucille-qa-runtime/lucille-synthetic-real22
```

As duas primeiras coordenações esperam condições de conclusão; monitorar logs e resultados negativos, sem deixar uma espera ocultar uma falha. Executar falhas de extração somente depois da série/lifecycle, com a GUI disponível e sem outra geração concorrente. Se uma tentativa perder a janela temporal, conservá-la como não comprovada; eventual repetição usa sufixo novo e registra todas as tentativas. A prova exige job realmente `running` e chamada real em andamento, não apenas um arquivo de marcador antigo.

O ensaio opcional próximo do limite deve ocorrer sozinho. Depois dos demais testes, encerrar somente GUI e Nomic próprios pode liberar recursos; Gemma conserva 65.536 tokens. A guarda pode registrar que não há margem para o ensaio. Não fechar aplicativos existentes, alterar pagefile ou reduzir contexto para conseguir um resultado.

## Medição e revisão

`summarize-phase22-extraction.py` separa propostas brutas, validação, candidatas e promoções por categoria. `summarize-phase22-resources.py` usa amostras reais e distingue dados medidos da projeção de mais 2 GiB. `package-phase22-evidence.py` publica somente arquivos selecionados de QA sintético, remove caminhos pessoais e registra hashes originais e públicos distintos.

A rubrica automática congelada não substitui inspeção literal. Nesta fase, ela deixou passar afirmações de implementação com sinônimos e gerou um falso alarme por palavra genérica. Os arquivos de revisão conservam o resultado original e cada ajuste. Uma pontuação agregada alta não aprova uma resposta crítica.

`phase22-final-github.py` faz uma única tentativa de dispatch no HEAD já publicado e uma auditoria limitada de leitura. Antes da chamada, grava a intenção para impedir repetição automática se houver interrupção. Nenhum helper altera permissões administrativas.

`executed-to-published.json` liga os scripts externos realmente executados às cópias publicadas. A adaptação modifica apenas a resolução de caminhos e o filtro de caminho do usuário atual. Os algoritmos de medição e os golds permanecem iguais.
