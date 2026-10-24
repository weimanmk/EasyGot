package io.github.weimanmk.easygot;

import android.app.NativeActivity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.database.Cursor;
import android.util.Base64;
import android.security.keystore.*;
import org.json.*;
import java.io.*;
import java.security.KeyStore;
import java.util.concurrent.ConcurrentLinkedQueue;
import javax.crypto.*;
import javax.crypto.spec.GCMParameterSpec;

/** System bridge only. UI, database, exams, import rules, AI scoring and sync are Rust. */
public final class MainActivity extends NativeActivity {
    private final ConcurrentLinkedQueue<String> events=new ConcurrentLinkedQueue<>();
    private String exportSource="";
    @Override protected void onCreate(Bundle state){super.onCreate(state);if(state!=null)exportSource=state.getString("exportSource","");}
    @Override protected void onSaveInstanceState(Bundle state){super.onSaveInstanceState(state);state.putString("exportSource",exportSource);}
    public String bridge(String command,String payload){
        try{switch(command){
            case "poll": {String event=events.poll();return event==null?"":event;}
            case "loadConfig": return loadConfig();
            case "saveConfig": saveConfig(payload);return "ok";
            case "pick": {int kind=Integer.parseInt(payload);runOnUiThread(()->{Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType(kind==3?"image/*":"*/*");startActivityForResult(i,kind);});return "ok";}
            case "export": {JSONObject p=new JSONObject(payload);File file=new File(p.getString("path"));if(!file.getCanonicalPath().startsWith(getFilesDir().getCanonicalPath()+"/"))throw new IOException("非法导出路径");exportSource=file.getAbsolutePath();String name=p.getString("name");runOnUiThread(()->{Intent i=new Intent(Intent.ACTION_CREATE_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType("application/json");i.putExtra(Intent.EXTRA_TITLE,name);startActivityForResult(i,20);});return "ok";}
            case "renderPdf": return renderPdf(payload);
            default: throw new IOException("未知系统桥接操作");
        }}catch(Exception e){return "ERROR:"+e.getClass().getSimpleName()+": "+e.getMessage();}
    }
    private java.security.Key key()throws Exception{KeyStore store=KeyStore.getInstance("AndroidKeyStore");store.load(null);if(!store.containsAlias("EasyGotConfig")){KeyGenerator g=KeyGenerator.getInstance("AES","AndroidKeyStore");g.init(new KeyGenParameterSpec.Builder("EasyGotConfig",KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());g.generateKey();}return store.getKey("EasyGotConfig",null);}
    private String loadConfig()throws Exception{String value=getSharedPreferences("local-vault",MODE_PRIVATE).getString("config","");if(value.isEmpty())return "{}";String[] parts=value.split(":");Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.DECRYPT_MODE,key(),new GCMParameterSpec(128,Base64.decode(parts[0],Base64.NO_WRAP)));return new String(cipher.doFinal(Base64.decode(parts[1],Base64.NO_WRAP)),java.nio.charset.StandardCharsets.UTF_8);}
    private void saveConfig(String value)throws Exception{Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.ENCRYPT_MODE,key());String encrypted=Base64.encodeToString(cipher.getIV(),Base64.NO_WRAP)+":"+Base64.encodeToString(cipher.doFinal(value.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Base64.NO_WRAP);if(!getSharedPreferences("local-vault",MODE_PRIVATE).edit().putString("config",encrypted).commit())throw new IOException("保存配置失败");}
    private void eventError(Exception e){events.add("ERROR:"+e.getMessage());}
    @Override protected void onActivityResult(int request,int result,Intent intent){super.onActivityResult(request,result,intent);if(result!=RESULT_OK||intent==null||intent.getData()==null)return;Uri uri=intent.getData();new Thread(()->{try{
        if(request==20){try(InputStream in=new FileInputStream(exportSource);OutputStream out=getContentResolver().openOutputStream(uri,"wt")){if(out==null)throw new IOException("无法写入文件");copy(in,out,100*1024*1024);}events.add("{\"kind\":20}");return;}
        String name="file";try(Cursor c=getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){if(c!=null&&c.moveToFirst())name=c.getString(0);}
        File dir=new File(getFilesDir(),"incoming");dir.mkdirs();File file=File.createTempFile("import-",".data",dir);try(InputStream in=getContentResolver().openInputStream(uri);OutputStream out=new FileOutputStream(file)){if(in==null)throw new IOException("无法读取文件");copy(in,out,request==4?100*1024*1024:30*1024*1024);}
        events.add(new JSONObject().put("kind",request).put("name",name).put("path",file.getAbsolutePath()).toString());
    }catch(Exception e){eventError(e);}},"easygot-documents").start();}
    private static void copy(InputStream in,OutputStream out,int max)throws IOException{byte[] buffer=new byte[8192];int n,total=0;while((n=in.read(buffer))!=-1){total+=n;if(total>max)throw new IOException("文件超过大小限制");out.write(buffer,0,n);}}
    private String renderPdf(String path)throws Exception{
        File source=new File(path);if(!source.getCanonicalPath().startsWith(getFilesDir().getCanonicalPath()+"/"))throw new IOException("非法PDF路径");JSONArray pages=new JSONArray();File dir=new File(getFilesDir(),"rendered");dir.mkdirs();
        try(android.os.ParcelFileDescriptor fd=android.os.ParcelFileDescriptor.open(source,android.os.ParcelFileDescriptor.MODE_READ_ONLY);android.graphics.pdf.PdfRenderer pdf=new android.graphics.pdf.PdfRenderer(fd)){
            if(pdf.getPageCount()>120)throw new IOException("视觉识别最多120页，请拆分文件");
            for(int i=0;i<pdf.getPageCount();i++)try(android.graphics.pdf.PdfRenderer.Page page=pdf.openPage(i)){int w=1400,h=page.getHeight()*w/page.getWidth();if(h>4000){h=4000;w=page.getWidth()*h/page.getHeight();}android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(w,h,android.graphics.Bitmap.Config.ARGB_8888);bitmap.eraseColor(android.graphics.Color.WHITE);page.render(bitmap,null,null,android.graphics.pdf.PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY);File file=File.createTempFile("page-",".jpg",dir);try(OutputStream out=new FileOutputStream(file)){bitmap.compress(android.graphics.Bitmap.CompressFormat.JPEG,85,out);}bitmap.recycle();pages.put(new JSONObject().put("path",file.getAbsolutePath()).put("locator","第 "+(i+1)+" 页（视觉识别）"));}
        }return pages.toString();
    }
}
