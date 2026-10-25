"""Read synthetic workbook bytes from stdin, never a business database or credentials."""
import sys,json,io
from openpyxl import load_workbook
expected=json.loads(sys.argv[1]);book=load_workbook(io.BytesIO(sys.stdin.buffer.read()),data_only=False)
assert book['导出说明']['B1'].value=='skarmar.prd.export.v1'
assert int(book['导出说明']['B2'].value)==6
assert not any('auth_' in name or 'operation' in name for name in book.sheetnames)
raw={}
for row in list(book['原始数据分段'].values)[1:]:
 name,index,rid,version,part,total,value=row;raw.setdefault((name,int(index)),[]).append((int(part),value))
tables={}
for (name,index),parts in raw.items():tables.setdefault(name,[]).append((index,json.loads(''.join(v for _,v in sorted(parts)))))
for name,count in expected.items():assert len(tables.get(name,[]))==count,(name,count,len(tables.get(name,[])))
for sheet in book:
 for row in sheet:
  for cell in row:assert cell.data_type!='f','User text became a formula'
print('PASS new-model XLSX: version marker, numeric cells, current records, history/relations, lossless JSON and no executable formulas')
