#!/usr/bin/env python3
"""Reproducible accuracy check of `tezaur audit` on a small hand-written benchmark (8 passages, 25 claims of five kinds).
It measures what the audit can and cannot see; it is NOT a general accuracy figure. Run: python3 audit_benchmark.py [path/to/tezaur]"""
import json, os, subprocess, sys, tempfile
DOCS = {'directive': 'Under Directive 2005/36/EC a host Member State may require compensation measures where the training of the migrant professional differs substantially from that required in the host State. The migrant may choose between an aptitude test and an adaptation period of up to three years for most professions. For lawyers the host State may impose the aptitude test.', 'lisbon': 'The Lisbon Recognition Convention requires parties to recognise higher education qualifications unless a substantial difference can be demonstrated by the competent authority. The burden of proof lies with the body assessing the qualification, not with the applicant. National information centres provide information on foreign education systems and support fair recognition.', 'imi': 'The Internal Market Information System is a secure multilingual online tool that lets competent authorities exchange information with counterparts in other Member States. It supports administrative cooperation for professional qualifications, services and posting of workers. Requests are routed to the authority responsible and answered within set deadlines.', 'bologna': 'The Bologna Process established the European Higher Education Area, built on three cycles: bachelor, master and doctorate. Credits are expressed in ECTS, where one academic year corresponds to sixty credits. Quality assurance follows the European Standards and Guidelines agreed by ministers.', 'procurement': 'In public procurement the contracting authority must treat economic operators equally and act transparently. Award criteria must be linked to the subject matter of the contract and published in advance. The most economically advantageous tender is determined on the basis of price or cost and may include quality criteria.', 'gdpr': 'Personal data must be processed lawfully, fairly and transparently and collected only for specified purposes. Controllers must be able to demonstrate compliance and must notify supervisory authorities of breaches within seventy-two hours where feasible. Data subjects have rights of access, rectification and erasure.', 'diploma': 'A diploma supplement describes the nature, level, context, content and status of the studies completed. It is issued automatically and free of charge in a widely spoken European language. It does not replace the original qualification and carries no automatic right of recognition.', 'ects': 'The European Credit Transfer System measures learning outcomes by workload. Typical student workload ranges from twenty-five to thirty hours per credit. Credits are awarded only when the learning outcomes have been achieved and assessed.'}
BIN = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "target", "release", "tezaur")
T = tempfile.mkdtemp(); os.makedirs(T + "/lib")
for k, v in DOCS.items(): open(T + "/lib/%s.txt" % k, "w").write(v)
env = dict(os.environ, TEZAUR_INDEX=T + "/lib/t.idx")
subprocess.run([BIN, "ingest", "."], cwd=T + "/lib", env=env, capture_output=True)
cases = [
# (kind, key, sentence, should_flag)
("verbatim","lisbon","The burden of proof lies with the body assessing the qualification.",False),
("verbatim","directive","The migrant may choose between an aptitude test and an adaptation period.",False),
("verbatim","imi","The system lets competent authorities exchange information with counterparts in other Member States.",False),
("verbatim","bologna","Credits are expressed in ECTS, where one academic year corresponds to sixty credits.",False),
("verbatim","procurement","Award criteria must be linked to the subject matter of the contract and published in advance.",False),
("verbatim","gdpr","Controllers must notify supervisory authorities of breaches within seventy-two hours.",False),
("paraphrase","lisbon","The assessing institution, rather than the applicant, must prove that a difference is substantial.",False),
("paraphrase","directive","Foreign-trained professionals can be asked to take a test or complete a supervised period when their training differs.",False),
("paraphrase","imi","Authorities in different countries share data through a protected web platform.",False),
("paraphrase","bologna","Europe adopted a degree structure of bachelor's, master's and doctoral levels.",False),
("paraphrase","procurement","Public buyers must be even-handed and open, and must announce how bids will be scored.",False),
("paraphrase","gdpr","Organisations have to report security incidents to the regulator in about three days.",False),
("contradiction","lisbon","The burden of proof lies with the applicant, not with the body assessing the qualification.",True),
("contradiction","directive","The host State may require an adaptation period of up to ten years for most professions.",True),
("contradiction","bologna","One academic year corresponds to thirty credits in the European Higher Education Area.",True),
("contradiction","gdpr","Controllers must notify supervisory authorities of breaches within seven days.",True),
("contradiction","diploma","The diploma supplement replaces the original qualification and carries an automatic right of recognition.",True),
("contradiction","ects","Credits are awarded for attendance regardless of whether learning outcomes were achieved.",True),
("unrelated","lisbon","Penguins breed on the Antarctic ice shelf during the austral winter.",True),
("unrelated","directive","Rust guarantees memory safety without a garbage collector.",True),
("unrelated","imi","The sonata form developed in the classical period of European music.",True),
("unrelated","procurement","Photosynthesis converts light energy into chemical energy in chloroplasts.",True),
("wrong-source","bologna","The burden of proof lies with the body assessing the qualification.",True),
("wrong-source","gdpr","The migrant may choose between an aptitude test and an adaptation period.",True),
("wrong-source","ects","Award criteria must be linked to the subject matter of the contract.",True),
]
res={}
for kind,key,s,should in cases:
    txt=f"{s} [@{key}]\n"; open(T + "/d.md","w").write(txt)
    out=json.loads(subprocess.run([BIN,"audit",T + "/d.md","--json"],env=env,capture_output=True,text=True).stdout)
    flagged = any(f["verdict"] not in ("supported",) for f in out)
    v = out[0]["verdict"] if out else "none"
    sup = out[0]["support"] if out else 0
    res.setdefault(kind,[]).append((flagged==should, v, sup, s[:60]))
for kind,l in res.items():
    print(f"{kind:14s} correct {sum(x[0] for x in l)}/{len(l)}   verdicts: {[x[1] for x in l]}")
    for x in l:
        if not x[0]: print("    MISS:", x[1], x[2], x[3])
