"""Bounded, isolated local benchmark. The gold strings are never sent to the model."""
import argparse
import hashlib
import json
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

EXTRACT = [
    ("Eu tomo chá. No conto, o narrador diz: 'Minha bebida favorita é café'.", "USER toma chá; preferência por café pertence ao narrador."),
    ("Para Aurora, escolhi SQLite. Pretendo implementar a busca em Rust.", "SQLite escolhido; Rust planejado; implementação/uso não comprovados."),
    ("Ontem implementei a busca de Íris em Go. Hoje a equipe já usa essa busca.", "Implementação ontem em Go; uso efetivo hoje pela equipe."),
    ("Corrigindo: os 32 GB são do portátil. O desktop passou de 48 para 64 GB.", "Portátil 32, desktop anterior 48, atual 64 GB."),
    ("Se eu comprasse um tablet, escolheria 16 GB. Ainda não comprei nenhum.", "Nenhuma compra; 16 GB hipotéticos."),
    ("A frase 'eu adoro interfaces cheias' é do personagem. Eu detesto interfaces carregadas.", "Personagem aprecia; USER rejeita interfaces carregadas."),
]
FACTS = "Escopo permitido: Lume. Desktop de Lume: antes 48 GB, agora 64 GB. Notebook de Lume: 12 GB. Lume escolheu SQLite. Implementação e uso do banco não documentados. Não há fatos autorizados sobre Sal."
ORACLE = [
    ("Quanta RAM tem hoje o desktop?", "64 GB"),
    ("Antes da troca, qual era a memória do desktop?", "48 GB"),
    ("Compare a memória atual do desktop e a do notebook.", "64 vs 12 GB"),
    ("Qual foi o banco escolhido e ele já está em uso?", "SQLite escolhido; uso desconhecido"),
    ("Qual é o banco de Sal?", "Desconhecido neste escopo"),
    ("Quanto tinha a máquina principal antes e quanto tem agora?", "48 antes, 64 agora"),
]
BASIC = "Responda em português, em até duas frases, usando os fatos fornecidos."
EXPLICIT = BASIC + " Respeite autoria, escopo e tempo. Indique quando faltar evidência."


def request(url, payload=None):
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(url, data=data, headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(req, timeout=90) as response:
        return json.load(response)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for arg in ("server", "model", "output"):
        parser.add_argument("--"+arg, type=Path, required=True)
    args=parser.parse_args()
    base="http://127.0.0.1:19437"
    parameters=dict(temperature=0.2, seed=42, max_tokens=160, stream=False,
                    chat_template_kwargs={"enable_thinking":False})
    cases=[]
    for i,(statement,gold) in enumerate(EXTRACT):
        cases.append((f"x{i}", "Identifique autor, entidade, propriedade, valor e situação de cada afirmação. Responda em até três frases.", statement, gold))
    for prompt_id,system in [("basic",BASIC),("explicit",EXPLICIT)]:
        for i,(question,gold) in enumerate(ORACLE):
            cases.append((f"{prompt_id}{i}", system, FACTS+"\n"+question, gold))
    for i in [0,3,5]:
        cases.append((f"repeat-x{i}",*cases[i][1:]))
    for i,variation in [(0,"Hoje, de quanta memória dispõe a máquina principal?"),(3,"A escolha do banco comprova que Lume já o utiliza? Qual foi a escolha?"),(4,"Dá para identificar o SGBD do outro projeto Sal?")]:
        cases.append((f"variant{i}", BASIC, FACTS+"\n"+variation,ORACLE[i][1]))
    command=[str(args.server),"-m",str(args.model),"-c","4096","-ngl","99","-np","1","--host","127.0.0.1","--port","19437","--reasoning","off"]
    def sha(path):
        with path.open("rb") as file:
            return hashlib.file_digest(file,"sha256").hexdigest()
    metadata=dict(parameters=parameters,context=4096,model_sha256=sha(args.model),server_sha256=sha(args.server))
    with args.output.open("x",encoding="utf8") as output, tempfile.TemporaryFile() as log:
        server=subprocess.Popen(command,stdout=log,stderr=log)
        try:
            for _ in range(120):
                if server.poll() is not None:
                    log.seek(0)
                    raise RuntimeError(log.read().decode(errors="replace")[-2000:])
                try:
                    request(base+"/health")
                    break
                except OSError:
                    time.sleep(1)
            else:
                raise TimeoutError("local server startup")
            for case,system,user,gold in cases:
                start=time.monotonic()
                resp=request(base+"/v1/chat/completions",dict(parameters,messages=[dict(role="system",content=system),dict(role="user",content=user)]))
                choice=resp["choices"][0]
                row=dict(id=case,system=system,user=user,gold=gold,answer=choice["message"]["content"],finish=choice["finish_reason"],usage=resp["usage"],seconds=round(time.monotonic()-start,3))
                output.write(json.dumps(dict(row,**metadata),ensure_ascii=False)+"\n")
                output.flush()
        finally:
            server.terminate()
            server.wait(timeout=30)


if __name__=="__main__":
    main()
