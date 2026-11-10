import subprocess,sys,time,os,resource,signal
T,S=sys.argv[1],sys.argv[2]; modes=sys.argv[3:]
for i,m in enumerate(modes):
    port=18100+i; srv=subprocess.Popen(['python3',f'{S}/evil_oai.py',m,str(port)]); time.sleep(0.7)
    d=f'{S}/oai_{m}'; os.system(f'rm -rf {d}; mkdir {d}')
    t=time.time()
    try:
        p=subprocess.run([T,'--index',d+'/i.idx','oai',f'http://127.0.0.1:{port}/oai','-o',d+'/out.bib'],cwd=d,capture_output=True,timeout=45,stdin=subprocess.DEVNULL,env={**os.environ,'NO_PROXY':'127.0.0.1','no_proxy':'127.0.0.1'})
        rc=p.returncode; msg=(p.stdout+p.stderr)[-200:].decode('utf8','replace').replace('\n',' | ')
    except subprocess.TimeoutExpired: rc='TIMEOUT>45s'; msg=''
    ru=resource.getrusage(resource.RUSAGE_CHILDREN); sz=os.path.getsize(d+'/out.bib') if os.path.exists(d+'/out.bib') else -1
    print(f'{m:11s} rc={rc} {time.time()-t:5.1f}s maxrss={ru.ru_maxrss//1024}MB out.bib={sz}B :: {msg}',flush=True)
    srv.kill()
