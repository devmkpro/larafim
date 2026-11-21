# Instalação

## Requisitos

- Windows 10 ou 11, 64 bits.
- WebView2 (já vem no Windows 11; no Windows 10 o instalador baixa se faltar).
- Para o PHP: o *Visual C++ Redistributable* da Microsoft. Se faltar, o Sentinel avisa e mostra o
  link oficial.

## Baixar

Os instaladores ficam na [página de releases](https://github.com/josevictoremiliano/sentinel/releases/latest).
Baixe o `Sentinel_<versão>_x64-setup.exe`.

Baixe **só** de lá. O Sentinel não é distribuído em outros sites.

## Conferir o download

Cada release traz o `SHA256SUMS.txt` e uma **atestação de origem** do GitHub. A atestação prova que o
instalador foi compilado pelo workflow público deste repositório, a partir do código da tag.

Pelo hash, no PowerShell:

```powershell
Get-FileHash .\Sentinel_0.1.0_x64-setup.exe -Algorithm SHA256
```

O resultado precisa ser igual à linha do arquivo no `SHA256SUMS.txt`.

Pela atestação, com o [GitHub CLI](https://cli.github.com/):

```powershell
gh attestation verify .\Sentinel_0.1.0_x64-setup.exe --repo josevictoremiliano/sentinel
```

## Instalar

O instalador:

- instala **só para o seu usuário** (em `%LOCALAPPDATA%\Programs`), sem pedir administrador;
- pergunta o idioma (português ou inglês; dá para trocar depois em *Configurações › Geral*);
- instala junto o `sentinel-shim.exe`, usado pelos atalhos de terminal.

### Aviso do SmartScreen

O instalador ainda não tem assinatura digital de código. Por isso o Windows pode mostrar
"O Windows protegeu o computador". Confira o download como explicado acima e clique em
*Mais informações › Executar assim mesmo*.

## Onde ficam os dados

Tudo o que o Sentinel baixa e cria fica em `%LOCALAPPDATA%\Sentinel`:

| Pasta | Conteúdo |
|---|---|
| `config.toml` | Preferências |
| `projects.toml` | Projetos importados e a configuração de cada um |
| `runtimes\` | PHP, Node, MariaDB, PostgreSQL, Mailpit, RustFS, Composer, Adminer |
| `data\` | Bancos de dados, storage S3, caixa do Mailpit, certificados locais |
| `backups\` | Backups de banco por projeto |
| `bin\` | Atalhos `php`/`composer`/`node` do terminal |
| `logs\` | Logs do próprio Sentinel |

**Nada é gravado dentro dos seus projetos**, exceto quando você pede. Por exemplo, ao aplicar um
serviço no `.env`, o Sentinel guarda antes uma cópia em `.env.sentinel.bak`.

## Atualizar

O Sentinel avisa quando sai uma versão nova. Ele confere a última release do GitHub ao abrir e a cada
6 horas, e o aviso aparece numa barra no topo da janela principal, no painel da bandeja, na dica do
ícone e em *Configurações › Sobre*. Lá também ficam o botão **Verificar atualizações** e a opção de
desligar a verificação automática.

O aviso só leva à página da release: baixe o instalador novo, confira o download e rode por cima.
Configurações, projetos e bancos são mantidos.

## Desinstalar

Use *Configurações do Windows › Aplicativos › Sentinel › Desinstalar*. O desinstalador remove o
programa e a preferência de idioma. Seus dados em `%LOCALAPPDATA%\Sentinel` **ficam**, para não
apagar bancos por engano. Para remover tudo:

1. Em *Configurações › Terminal* do Sentinel, desligue os atalhos no PATH (antes de desinstalar).
2. Desinstale.
3. Apague a pasta `%LOCALAPPDATA%\Sentinel`.
4. Se tiver confiado no certificado HTTPS local, remova-o em `certmgr.msc` ›
   *Autoridades de Certificação Raiz Confiáveis* (procure por "Sentinel").
5. Se quiser, apague o bloco `# BEGIN SENTINEL` … `# END SENTINEL` do arquivo
   `C:\Windows\System32\drivers\etc\hosts`.
