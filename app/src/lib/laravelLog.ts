import { t } from "./i18n.svelte";

// Lê o formato de log do Laravel (Monolog LineFormatter):
//   [2026-11-21 10:00:00] local.ERROR: Mensagem {"contexto":...}
//   [stacktrace]
//   #0 C:\www\app\vendor\...\Arquivo.php(12): Classe->metodo()
// Linhas que não começam com "[data]" continuam a entrada anterior.
// Logs de outro formato viram entradas sem nível, uma por linha.

export type Level = "error" | "warning" | "info" | "debug" | "other";

export interface Frame {
  n: number;
  call: string;
  file: string | null;
  line: number | null;
  vendor: boolean;
}

export interface LogException {
  cls: string;
  code: string;
  message: string;
  file: string | null;
  line: number | null;
}

export interface LogEntry {
  id: number;
  /** "2026-11-21 10:00:00" */
  datetime: string;
  time: string;
  env: string;
  level: Level;
  levelName: string;
  message: string;
  context: string | null;
  exception: LogException | null;
  frames: Frame[];
  raw: string;
}

const HEAD = /^\[(\d{4}-\d{2}-\d{2}[T ][\d:.]+)(?:[+-]\d{2}:?\d{2}|Z)?\] ([\w-]+)\.(\w+): ?(.*)$/;

export function levelOf(name: string): Level {
  switch (name.toUpperCase()) {
    case "EMERGENCY":
    case "ALERT":
    case "CRITICAL":
    case "ERROR":
      return "error";
    case "WARNING":
      return "warning";
    case "NOTICE":
    case "INFO":
      return "info";
    case "DEBUG":
      return "debug";
    default:
      return "other";
  }
}

const isVendor = (path: string) => /[\\/]vendor[\\/]/i.test(path);

/** O Laravel 11+ grava namespaces com "/" (Illuminate/Http/Request): volta para "\". */
const ns = (s: string) => s.replace(/(\w)\/(?=[A-Z_])/g, "$1\\");

function parseFrame(n: number, text: string): Frame {
  // "C:\...\Arquivo.php(12): Classe->metodo()" | "[internal function]: f()" | "{main}"
  const m = /^(.+?)\((\d+)\): (.*)$/.exec(text);
  if (m) return { n, file: m[1], line: Number(m[2]), call: ns(m[3]), vendor: isVendor(m[1]) };
  const internal = /^\[internal function\]: (.*)$/.exec(text);
  if (internal) return { n, file: null, line: null, call: ns(internal[1]), vendor: true };
  return { n, file: null, line: null, call: text, vendor: false };
}

/** No JSON do contexto, as barras do Windows vêm dobradas. Uma passada só:
 *  `\\"` vira `\"` (barra + aspa), e não `"` (desfazer duas vezes). */
const unescape = (s: string) => s.replace(/\\(["\\/])/g, "$1");

function finish(id: number, datetime: string, env: string, levelName: string, lines: string[]): LogEntry {
  const body = lines.join("\n");
  // Mensagem = tudo antes do contexto JSON (` {"…` ou ` […`), que pode ter várias linhas.
  let message = body;
  let context: string | null = null;
  const ctx = body.search(/ [{[]["\]{]/);
  if (ctx >= 0) {
    message = body.slice(0, ctx);
    context = body.slice(ctx + 1);
  }
  message = message.trim();

  let exception: LogException | null = null;
  const ex = /\[object\] \(([^\s(]+)\(code: ([^)]*)\): ([\s\S]*?) at ((?:[A-Za-z]:)?[^:\n]+):(\d+)\)/.exec(body);
  if (ex) {
    exception = { cls: ns(unescape(ex[1])), code: ex[2], message: unescape(ex[3]).trim(), file: unescape(ex[4]), line: Number(ex[5]) };
  }

  // Só a primeira pilha (a da exceção principal).
  const frames: Frame[] = [];
  const st = body.indexOf("[stacktrace]");
  if (st >= 0) {
    for (const l of body.slice(st + 12).split("\n")) {
      const m = /^#(\d+) (.*)$/.exec(l.trim());
      if (!m) {
        if (frames.length) break;
        continue;
      }
      frames.push(parseFrame(Number(m[1]), unescape(m[2]).replace(/"}?\s*$/, "")));
    }
  }
  if (exception && context?.startsWith('{"exception":"[object]')) context = null;

  return {
    id,
    datetime: datetime.replace("T", " ").replace(/\.\d+$/, ""),
    time: (datetime.split(/[T ]/)[1] ?? "").replace(/\.\d+$/, ""),
    env,
    level: levelOf(levelName),
    levelName: levelName.toUpperCase(),
    message: message || exception?.message || "",
    context: context?.trim() || null,
    exception,
    frames,
    raw: `[${datetime}] ${env}.${levelName}: ${body}`,
  };
}

/** `firstId` mantém os ids estáveis entre leituras. */
export function parseLog(text: string, firstId = 0): LogEntry[] {
  const out: LogEntry[] = [];
  let cur: { datetime: string; env: string; level: string; lines: string[] } | null = null;
  let id = firstId;
  const flush = () => {
    if (cur) out.push(finish(id++, cur.datetime, cur.env, cur.level, cur.lines));
    cur = null;
  };
  for (const line of text.split(/\r?\n/)) {
    const m = HEAD.exec(line);
    if (m) {
      flush();
      cur = { datetime: m[1], env: m[2], level: m[3], lines: [m[4]] };
    } else if (cur) {
      cur.lines.push(line);
    } else if (line.trim()) {
      // Formato desconhecido: uma entrada por linha.
      out.push({
        id: id++,
        datetime: "",
        time: "",
        env: "",
        level: /\b(error|fatal|exception)\b/i.test(line) ? "error" : /\bwarn(ing)?\b/i.test(line) ? "warning" : "other",
        levelName: "",
        message: line,
        context: null,
        exception: null,
        frames: [],
        raw: line,
      });
    }
  }
  flush();
  // Linhas em branco no fim de cada entrada não contam.
  for (const e of out) e.context = e.context?.replace(/\s+$/, "") || null;
  return out;
}

/** Caminho relativo ao projeto, quando estiver dentro dele. */
export function relative(file: string, project: string): string {
  const norm = (s: string) => s.replace(/\//g, "\\").toLowerCase();
  const base = norm(project).replace(/\\$/, "") + "\\";
  return norm(file).startsWith(base) ? file.slice(base.length).replace(/\\/g, "/") : file;
}

/** Texto em Markdown para colar numa issue ou num chat. */
export function toMarkdown(e: LogEntry, project: string): string {
  const out = [`**${e.levelName || "LOG"}** ${e.datetime}`, "", e.message];
  if (e.exception) {
    const cls = "`" + e.exception.cls + "`";
    out.push(
      "",
      e.exception.file
        ? t("{cls} em {file}", { cls, file: "`" + relative(e.exception.file, project) + ":" + e.exception.line + "`" })
        : cls,
    );
  }
  if (e.frames.length) {
    out.push("", "```");
    for (const f of e.frames) out.push(`#${f.n} ${f.file ? `${relative(f.file, project)}(${f.line}): ` : ""}${f.call}`);
    out.push("```");
  } else if (e.context) {
    out.push("", "```json", e.context, "```");
  }
  return out.join("\n");
}
