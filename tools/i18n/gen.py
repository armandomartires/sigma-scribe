import sys,re
names={'de':'Deutsch','es':'Español','it':'Italiano','nl':'Nederlands','ca':'Català','da':'Dansk','sv':'Svenska','fi':'Suomi','nb':'Norsk','tr':'Türkçe','pl':'Polski','cs':'Čeština','ro':'Română','ru':'Русский','uk':'Українська','el':'Ελληνικά','id':'Bahasa Indonesia','ms':'Bahasa Melayu','fil':'Filipino'}
ks=open('tools/i18n/keys.txt').read().rstrip('\n').split('\n')
for c in sys.argv[1:]:
    vs=open(f'tools/i18n/src/{c}.txt').read().rstrip('\n').split('\n')
    assert len(vs)==len(ks),(c,len(vs))
    for i,(k,v) in enumerate(zip(ks,vs)):
        ph=lambda s: sorted(re.findall(r'\{\w+\}',s))
        assert ph(k)==ph(v),(c,i+1,k,v)
        assert v.strip(),(c,i+1)
    out=[f"# Sigma Scribe — {names[c]}. machine-drafted, best effort; needs a native review.",'# Format: English text, then "= " and the translation. See crates/render/src/i18n.rs.','']
    for k,v in zip(ks,vs): out+= [k,'= '+v,'']
    open(f'assets/i18n/{c}.txt','w').write('\n'.join(out))
    print(c,'ok')
