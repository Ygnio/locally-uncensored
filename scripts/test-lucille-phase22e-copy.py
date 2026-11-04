"""Click own QA Copy action, preserving all existing HGLOBAL clipboard formats in RAM."""
import ctypes,json,time,uuid,sys,re,importlib.util
from ctypes import wintypes as w
from pathlib import Path
u=ctypes.WinDLL('user32',use_last_error=True);k=ctypes.WinDLL('kernel32',use_last_error=True)
u.CreateWindowExW.argtypes=[w.DWORD,w.LPCWSTR,w.LPCWSTR,w.DWORD,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_int,w.HWND,w.HMENU,w.HINSTANCE,w.LPVOID];u.CreateWindowExW.restype=w.HWND
u.OpenClipboard.argtypes=[w.HWND];u.GetClipboardData.argtypes=[w.UINT];u.GetClipboardData.restype=w.HANDLE
u.SetClipboardData.argtypes=[w.UINT,w.HANDLE];u.SetClipboardData.restype=w.HANDLE
k.GlobalSize.argtypes=[w.HGLOBAL];k.GlobalSize.restype=ctypes.c_size_t
k.GlobalLock.argtypes=[w.HGLOBAL];k.GlobalLock.restype=w.LPVOID;k.GlobalUnlock.argtypes=[w.HGLOBAL]
k.GlobalAlloc.argtypes=[w.UINT,ctypes.c_size_t];k.GlobalAlloc.restype=w.HGLOBAL
root=Path('.lucille-qa-runtime/lucille-synthetic-real22e').resolve();assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
reports=root/'reports';assert not (reports/'phase22e-controller.lock').exists()
spec=importlib.util.spec_from_file_location('artifact_check',Path('qa/phase22e-artifact-check.py'));artifacts=importlib.util.module_from_spec(spec);spec.loader.exec_module(artifacts)
label=sys.argv[1];assert re.fullmatch(r'(?:development|final)-[a-z0-9-]{1,38}',label)
out=reports/('driver-'+label+'-copy.json');assert not out.exists()
window=u.CreateWindowExW(0,'STATIC','Isolated QA clipboard preservation',0,0,0,0,0,w.HWND(-3),None,None,None);assert window
u.CopyImage.argtypes=[w.HANDLE,w.UINT,ctypes.c_int,ctypes.c_int,w.UINT];u.CopyImage.restype=w.HANDLE
g=ctypes.WinDLL('gdi32',use_last_error=True);g.DeleteObject.argtypes=[w.HANDLE]
saved=[];bitmaps=[];restored=False;changed=False
try:
 assert u.OpenClipboard(window)
 try:
  sequence=u.GetClipboardSequenceNumber();fmt=0
  while (fmt:=u.EnumClipboardFormats(fmt)):
   handle=u.GetClipboardData(fmt)
   if fmt==2:
    duplicate=u.CopyImage(handle,0,0,0,0x2000);assert duplicate
    saved.append((fmt,duplicate));bitmaps.append(duplicate);continue
   size=k.GlobalSize(handle)
   assert 0<size<32*1024*1024,'Clipboard format cannot be safely preserved; Copy test not sent'
   address=k.GlobalLock(handle);assert address
   try:saved.append((fmt,ctypes.string_at(address,size)))
   finally:k.GlobalUnlock(handle)
 finally:u.CloseClipboard()
 assert u.GetClipboardSequenceNumber()==sequence,'Clipboard changed before Copy test; test not sent'
 (reports/'control.json').write_bytes((json.dumps({'id':str(uuid.uuid4()),'label':label,'copy':True})+'\n').encode())
 result=artifacts.read_report(out,35)
 if result['status']=='system_write_resolved':
  changed=True;assert u.OpenClipboard(window)
  try:
   h=u.GetClipboardData(13);address=k.GlobalLock(h);assert address
   try:current=ctypes.wstring_at(address)
   finally:k.GlobalUnlock(h)
   if current==result['verified_text']:
    assert u.EmptyClipboard()
    for fmt,data in saved:
     if fmt==2:
      assert u.SetClipboardData(fmt,data);bitmaps.remove(data);continue
     handle=k.GlobalAlloc(0x0002,len(data));assert handle
     address=k.GlobalLock(handle);assert address
     ctypes.memmove(address,data,len(data));k.GlobalUnlock(handle)
     assert u.SetClipboardData(fmt,handle)
    restored=True
  finally:u.CloseClipboard()
 report={'status':result['status'],'formats_preserved_in_ram':len(saved),'clipboard_restored':restored,'clipboard_write_observed':changed,'personal_clipboard_content_logged':False}
 audit=reports/(label+'-clipboard-preservation.json');assert not audit.exists();audit.write_bytes((json.dumps(report,indent=2)+'\n').encode());print(json.dumps(report))
finally:
 for bitmap in bitmaps:g.DeleteObject(bitmap)
 u.DestroyWindow.argtypes=[w.HWND];u.DestroyWindow(window)
