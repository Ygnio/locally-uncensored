# Lucille 2.1D: critérios fixos de aceitação

Registrados antes da avaliação inédita. Conjuntos históricos e de desenvolvimento não são avaliações inéditas. Os critérios vêm da missão do usuário, sem alteração por resultados.

- Extração: regras, Gemma isolado e híbrido separados; precisão ≥95%, recall ≥80%. Propostas brutas, validação e promoção com contagens distintas. Zero gravações críticas de fonte insegura, escopo, identidade ou permissões.
- Recuperação: FTS5, Nomic e híbrido separados; híbrido de produção com precisão do contexto ≥90% e recall ≥80%. Zero excluídas/obsoletas como atuais, violações críticas de escopo ou contexto em perguntas claramente irrelevantes. Ranking não equivale a contexto injetado.
- GUI real isolada: Chat/Agent/Code, revisão, indexação, paráfrase e resposta, correção/reindexação, exclusão/reaprendizagem e reinício. Naturalidade apenas preliminar.
- Preservar instalação e dados operacionais, modelos e aplicativos existentes. Gemma 65536, Nomic CPU 768/contexto2048. Recursos A medidos; B +2GiB estritamente projetado.
- Congelar fontes, prompt, schema, normalizador, política, parâmetros e corpora antes da primeira execução final. Não ajustar a avaliação após resultados. Se gates falharem, registrar parcial e conservar negativos.
- Publicar branch e PR rascunho contra 2.1C; sem merge/release/instalação. Auditar staging e uma tentativa de CI no HEAD final, sem permissões administrativas.

## Desenvolvimento ainda em andamento

O perfil candidato usa IR V2 com evidências completas selecionadas dentre todas as cláusulas da fonte; interpretação independente e revisão obrigatória. Nenhuma evidência ausente é completada. Representações de índice são derivadas da versão canônica atual. A configuração e os hashes finais serão registrados no manifesto congelado após desenvolvimento e antes dos corpora inéditos serem executados.
