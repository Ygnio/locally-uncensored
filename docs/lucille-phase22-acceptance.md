# Lucille 2.2 — critérios prévios à avaliação inédita

Base PR8 / 4116a9395582adf56c40100109783f67594c45b8. Os resultados anteriores são regressões, nunca avaliação independente. Aprendizado/autonomia novos desligados por padrão; ativação só em QA sintético por configuração explícita. Nenhuma alteração operacional, modelos/launchers originais, drivers ou pagefile; não encerrar aplicativos existentes.

## Gates fixos

Extração validada: P≥95%, R≥80%; promoção automática: P≥99% e zero indevidas críticas; recuperação contextual: P≥90%, R≥80%; zero violações críticas de escopo, autoridade ou dados excluídos/obsoletos como atuais; zero fusões incorretas no conjunto crítico. Informar raw/propostas, validação, promoção, contexto e resposta separadamente. Categorias com menos de 5 exemplos: contagens, sem conclusão percentual generalizada.

Nível A: preferências explícitas, não sensíveis, canônicas, sem ambiguidade, com prova independente reavaliada e política opt-in. Confiança numérica do modelo não autoriza promoção. Correção A somente com vínculo único explícito e snapshot/versionamento. B: hardware/projetos/objetivos/tarefas/restrições, ambiguidades e conflitos sem correção explícita ficam em revisão. C: fontes inseguras/terceiros/ficção/segredos/autoridade/escopo divergente rejeitadas.

Deduplicar apenas entidade/propriedade/valor compatíveis no mesmo escopo/projeto; relacionar gêneros sem fundir por similaridade. Esquecimento explícito do usuário deve ser idempotente, transacional, revogar trabalhos pendentes e invalidar índice; antigas interações não podem apagar novas afirmações nem recriar excluídas. Retomada explícita da preferência excluída exige nova fonte + revisão; não remover tombstones antigos.

## Rubrica objetiva de naturalidade (antes do ensaio final)

Cada resposta recebe 0/1 por critério aplicável: (1) utiliza corretamente a informação solicitada e fundamentada; (2) só apresenta versões atuais, sem extrapolar hipóteses; (3) não anuncia lembrança gratuitamente (permitido quando perguntado sobre memória/proveniência; termos técnicos como memória RAM não contam); (4) não repete preferência mais de uma vez nem a injeta em assunto irrelevante; (5) responde à pergunta irrelevante sem contaminação de memória. Aprovação: ≥90% dos itens aplicáveis, além de zero fatos críticos inventados/obsoletos/excluídos ou de outro escopo. Relatar por cenário, incluindo memória recuperada mas ignorada. Aplicação por assertions e inspeção das respostas reais; não usar juiz LLM como substituto do gold.

## Conversas e recursos

Ensaios crescentes 20/50/100 turnos e 200 se houver margem; novas sessões sem histórico comprovam persistência, com referências/versionamento no payload real. Reiniciar host e serviço Nomic próprios. Um ensaio próximo de 65536 tokens será tentado somente com margem, com contagem real do tokenizer; eventual impossibilidade é limitação, não aprovação. Gemma permanece 65536; Nomic CPU/768/contexto2048.

Instrumentar RAM/commit/pagefile/VRAM, latências, tamanhos SQLite/índice/fila, recuperação e crescimento. Evitar builds pesados com inferência. A é medição; B soma apenas a estimativa de +2GiB de RAM, sem alegar latência/commit/VRAM medidos. Guardas existentes continuam. Manter datasets/código/prompt/schema/política congelados antes da primeira avaliação inédita e conservar todo resultado negativo. Mudanças após falha exigem conjuntos novos, ou resultado parcial.

CI: uma tentativa no HEAD final, depois auditoria somente leitura. HTTP422 e API enabled:true anteriores não autorizam mudanças administrativas nem CI verde. Publicar commits e PR draft empilhado em D, sem merge/release/instalação. Todos os critérios de missão precisam de evidência; caso falhe um, declarar parcial.
