# HTTPS e domínios

## O domínio `.offline`

Cada projeto responde em `http://<domínio>.offline`. O sufixo é configurável em
*Configurações › Rede e domínios*.

Por que `.offline`: Herd, Valet e Laragon usam `.test`, então um sufixo diferente evita conflito
quando mais de um desses programas está instalado. `.offline` não é um domínio público, segundo a lista
da IANA, e o arquivo `hosts` tem prioridade sobre a internet de qualquer forma.

Dica: como `.offline` não é um domínio conhecido, digitar só `loja.offline` na barra do navegador pode
virar uma busca. Use o endereço completo (`http://loja.offline`) na primeira vez; o Sentinel sempre
abre o endereço completo.

## O arquivo `hosts`

Para o Windows saber que `loja.offline` é este PC, o Sentinel mantém um bloco no arquivo
`C:\Windows\System32\drivers\etc\hosts`:

```
# BEGIN SENTINEL
127.0.0.1 loja.offline
127.0.0.1 db.offline
# END SENTINEL
```

- Alterar esse arquivo exige administrador. O Sentinel pede (UAC) **só quando a lista de domínios
  muda**. Ligar e desligar projetos já importados não pede de novo.
- Quem grava é um pequeno ajudante elevado que **revalida cada linha**: só aceita
  `127.0.0.1 <domínio válido>`, no máximo 2000 linhas, e nunca mexe fora do bloco.
- Antes de gravar, ele guarda uma cópia em `hosts.sentinel-backup`.

## HTTPS local

Em *Configurações › Rede e domínios › HTTPS*:

1. Ligue **HTTPS**. O Sentinel cria uma autoridade certificadora (CA) só sua e passa a responder também
   em `https://` (porta 443).
2. Clique em **Confiar…** e confirme a pergunta do Windows. A CA é instalada **só no seu usuário**,
   não no computador todo.

Como o certificado é protegido:

- A CA é **restrita ao domínio local** por *name constraints*: mesmo que alguém roubasse a chave, ela
  não serviria para criar certificados de `google.com` ou de qualquer outro site real.
- A chave privada da CA fica protegida pela DPAPI do Windows, legível só pelo seu usuário.
- Cada projeto recebe o seu certificado na hora, sem guardar nada em disco.

O PHP recebe `HTTPS=on` e o Node recebe `X-Forwarded-Proto: https`, então o Laravel gera links `https://`.

### Firefox e ferramentas de linha de comando

- O **Firefox** tem a própria lista de CAs: importe o `.crt` de `%LOCALAPPDATA%\Sentinel\data\ca\`
  ou ligue `security.enterprise_roots.enabled` em `about:config`.
- O `curl` do Windows, o PowerShell e o .NET checam revogação e falham com certificados locais. Use
  `curl --ssl-no-revoke` ou equivalente.

## Portas

| Serviço | Porta padrão |
|---|---|
| Gateway HTTP | 80 |
| Gateway HTTPS | 443 |
| MariaDB | 3306 (ou a escolhida) |
| PostgreSQL | 5432 |
| Mailpit (SMTP) | 1025 |
| Storage S3 (API) | 9000 |

Se a porta 80 estiver ocupada (Laragon, IIS, Apache), o Sentinel mostra **quem** ocupa e permite usar
outra porta (os endereços passam a ter `:porta`). Tudo escuta só em `127.0.0.1`.
