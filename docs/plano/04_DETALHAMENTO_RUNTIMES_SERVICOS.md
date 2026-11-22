# Larafim — Especificação: runtimes, serviços e ferramentas

## PHP — Fase 1
**Catálogo:** versões minor, patch instalado, atualização disponível, projetos vinculados, instalar/atualizar/remover.
**Instalação:** baixar de fonte oficial, verificar SHA-256, extrair build NTS x64, gerar php.ini, exibir progresso/erro/repetir.
**Detalhes:** extensões ativadas, toggles, editor php.ini com validação e opção restaurar; atualização preserva versão anterior para rollback.
**Proteção:** não remover versão em uso sem informar dependências e confirmar resolução.

## Gateway — Fase 3
**Painel:** status, porta HTTP, domínios, processos conflitantes e instruções de correção.
**Ações:** iniciar/parar quando aplicável, testar, escolher porta alternativa, solicitar atualização de hosts com UAC.
**Estados:** porta ocupada, acesso negado, resolução ausente, projeto inacessível.

## CLI e Node — Fases 4–5
**CLI:** status dos shims, PATH, teste de versão contextual e reparo.
**Node:** catálogo de versões, instalação com hash, comando npm/pnpm/yarn, porta e logs por projeto.
**Laravel + Vite:** botão iniciar/parar Vite sem alterar o gateway PHP principal.

## MariaDB e bancos — Fase 6
**Página de serviço:** instalar, iniciar/parar, porta, modo leve/padrão, projetos dependentes, consumo de recursos.
**Banco por projeto:** criar banco, listar, abrir Adminer, exportar/importar SQL, histórico de backups.
**Variáveis .env:** exibir DB_* com porta real; copiar, visualizar diff, confirmar, criar backup e aplicar apenas chaves do serviço; desfazer.
**Estados:** porta ocupada, banco não inicializado, importação inválida, backup falhou, banco em uso.

## Ferramentas — Fase 7 e opcionais
**Catálogo:** Mailpit, Adminer, Xdebug, PostgreSQL, MinIO; instalado/não instalado, instalar/remover, ligar/desligar conforme aplicável.
**Mailpit:** status SMTP, URL `http://mail.offline`, copiar/aplicar MAIL_*.
**Adminer:** abrir `http://db.offline` após confirmar serviço disponível.
**Xdebug:** escolher versão PHP, baixar DLL compatível, ativar/desativar, mostrar configuração de editor.
**Workers:** queue:work, schedule:work, reverb:start; start/stop, status, reinício em falha e logs.
**Novo Laravel:** nome, pasta, PHP, banco, starter kit, resumo e progresso de criação.
**PostgreSQL/MinIO:** instalação opcional, configurações por projeto, backups Postgres, bucket S3 por projeto.

## Terminal, logs e monitor
**Terminal:** abrir na pasta do projeto, com versão contextual; carregar somente ao entrar na aba.
**Logs:** filtros por projeto/serviço, pausar acompanhamento, copiar trecho, abrir arquivo.
**Monitor:** CPU/RAM por processo Larafim, consumo e ações de desligamento.
**Tray:** status geral, projetos online, iniciar/parar geral, abrir janela, sair.

## Configurações — Fases 0–8
**Abas:** Geral; Pastas; Rede; Runtimes; Inicialização; Diagnóstico; Atualizações.
**Campos:** TLD padrão, portas, pastas, caminhos de armazenamento, iniciar com Windows, minimizar para bandeja.
**Avançado:** HTTPS local e certificados, DNS/NRPT, atualização do app.
**Regras:** avisar sobre efeitos em projetos ativos; pedir UAC apenas quando necessário; alterações destrutivas com confirmação.
