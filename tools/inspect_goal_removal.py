"""Read-only diagnosis using an existing application identity; never prints data."""
import hashlib,json,os
from pathlib import Path
from urllib.request import Request
from restore_demo_batch import Client
out={'state':'application_identity_not_configured'}
if os.environ.get('SKARMA_TEST_LOGIN','').strip():
 try:
  api=Client('https://skarma.vip');api.login()
  req=Request('https://skarma.vip/api/prd/state',headers={'User-Agent':'SkarmaGoalInspection/1.0','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'})
  with api.opener.open(req,timeout=30) as response:data=json.load(response)
  assert data['schema']=='skarmar.prd.v1'
  selectors=['8f543e798a94b145efdd6f6dd789fa52ec125cef82beb217b34fa09f91f12390','50c3fa9078406f542526ea1833cc608fb856c81c948ad1016314d48dea87c3fa']
  groups=[[g for g in data['goals'] if g['input']['level']!='long' and hashlib.sha256(g['input']['title'].encode()).hexdigest()==h] for h in selectors]
  out={'state':'candidate_missing' if any(not g for g in groups) else 'candidate_ambiguous' if any(len(g)!=1 for g in groups) else 'candidates_exist_guard_needs_reference_check'}
 except Exception:
  out={'state':'application_identity_or_read_unavailable'}
path=Path('.local/goal-removal-inspection');path.mkdir(parents=True,exist_ok=True)
(path/'result.json').write_text(json.dumps(out)+'\n');print(json.dumps(out))
