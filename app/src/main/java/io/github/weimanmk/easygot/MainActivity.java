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
    @Override protected void onCreate(Bundle state){super.onCreate(state);
        if(android.os.Build.VERSION.SDK_INT>=30)getWindow().setDecorFitsSystemWindows(false);
        else getWindow().getDecorView().setSystemUiVisibility(android.view.View.SYSTEM_UI_FLAG_LAYOUT_STABLE|android.view.View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN|android.view.View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION|android.view.View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR|android.view.View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        if(state!=null)exportSource=state.getString("exportSource","");}
    @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);if((getApplicationInfo().flags&android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE)==0)return;String command=intent.getStringExtra("qaScreen");if(command==null)return;try{if(command.equals("native-paste")){android.content.ClipboardManager clipboard=(android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE);clipboard.setPrimaryClip(android.content.ClipData.newPlainText("QA synthetic key","easygot-test-key"));bridge("editSecret",new JSONObject().put("field","aiKey").put("label","API Key").put("value","").toString());}else events.add(new JSONObject().put("kind",90).put("screen",command).toString());}catch(Exception ignored){}}
    @Override protected void onSaveInstanceState(Bundle state){super.onSaveInstanceState(state);state.putString("exportSource",exportSource);}
    public String bridge(String command,String payload){
        try{switch(command){
            case "background": runOnUiThread(()->moveTaskToBack(true));return "ok";
            case "poll": {String event=events.poll();return event==null?"":event;}
            case "loadConfig": return loadConfig();
            case "saveConfig": saveConfig(payload);return "ok";
            case "pick": {int kind=Integer.parseInt(payload);runOnUiThread(()->{Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType((kind==3||kind==5||kind==6)?"image/*":"*/*");if(kind==5||kind==6)i.putExtra(Intent.EXTRA_ALLOW_MULTIPLE,true);startActivityForResult(i,kind);});return "ok";}
            case "export": {JSONObject p=new JSONObject(payload);File file=new File(p.getString("path"));if(!file.getCanonicalPath().startsWith(getFilesDir().getCanonicalPath()+"/"))throw new IOException("非法导出路径");exportSource=file.getAbsolutePath();String name=p.getString("name");runOnUiThread(()->{Intent i=new Intent(Intent.ACTION_CREATE_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType(name.endsWith(".pdf")?"application/pdf":"application/json");i.putExtra(Intent.EXTRA_TITLE,name);startActivityForResult(i,20);});return "ok";}
            case "renderPdf": return renderPdf(payload);
            case "insets": {android.view.WindowInsets insets=getWindow().getDecorView().getRootWindowInsets();float density=getResources().getDisplayMetrics().density;if(insets==null)return "{}";int top=insets.getSystemWindowInsetTop(),bottom=insets.getSystemWindowInsetBottom();if(android.os.Build.VERSION.SDK_INT>=30){android.graphics.Insets bars=insets.getInsets(android.view.WindowInsets.Type.systemBars()|android.view.WindowInsets.Type.displayCutout());top=bars.top;bottom=bars.bottom;}int ime=0;if(android.os.Build.VERSION.SDK_INT>=30)ime=insets.getInsets(android.view.WindowInsets.Type.ime()).bottom;return new JSONObject().put("top",top/density).put("bottom",bottom/density).put("ime",ime/density).put("fontScale",getResources().getConfiguration().fontScale).toString();}
            case "clipboard": {android.content.ClipboardManager clipboard=(android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE);if(clipboard==null||!clipboard.hasPrimaryClip())return "";android.content.ClipData clip=clipboard.getPrimaryClip();return clip==null||clip.getItemCount()==0?"":clip.getItemAt(0).coerceToText(this).toString();}
            case "editSecret": {JSONObject spec=new JSONObject(payload);String field=spec.getString("field"),value=spec.optString("value"),label=spec.optString("label","输入密钥");runOnUiThread(()->{android.widget.EditText input=new android.widget.EditText(this);input.setInputType(android.text.InputType.TYPE_CLASS_TEXT|android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD);input.setSingleLine(true);input.setText(value);input.setSelectAllOnFocus(true);input.setPadding(40,30,40,30);new android.app.AlertDialog.Builder(this).setTitle(label).setView(input).setNegativeButton("取消",null).setPositiveButton("确定",(d,w)->{try{events.add(new JSONObject().put("kind",21).put("field",field).put("value",input.getText().toString().trim()).toString());}catch(JSONException ignored){}}).show();input.requestFocus();});return "ok";}
            case "printPaper": return PaperPrinter.create(this,payload);
            case "openPdf": {File pdf=new File(payload);if(!pdf.getCanonicalPath().startsWith(getFilesDir().getCanonicalPath()+"/"))throw new IOException("非法文件路径");runOnUiThread(()->{android.print.PrintManager manager=(android.print.PrintManager)getSystemService(PRINT_SERVICE);manager.print("EasyGot试卷",new PdfPrintAdapter(pdf),null);});return "ok";}
            default: throw new IOException("未知系统桥接操作");
        }}catch(Exception e){return "ERROR:"+e.getClass().getSimpleName()+": "+e.getMessage();}
    }
    private java.security.Key key()throws Exception{KeyStore store=KeyStore.getInstance("AndroidKeyStore");store.load(null);if(!store.containsAlias("EasyGotConfig")){KeyGenerator g=KeyGenerator.getInstance("AES","AndroidKeyStore");g.init(new KeyGenParameterSpec.Builder("EasyGotConfig",KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());g.generateKey();}return store.getKey("EasyGotConfig",null);}
    private String loadConfig()throws Exception{String value=getSharedPreferences("local-vault",MODE_PRIVATE).getString("config","");if(value.isEmpty())return "{}";String[] parts=value.split(":");Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.DECRYPT_MODE,key(),new GCMParameterSpec(128,Base64.decode(parts[0],Base64.NO_WRAP)));return new String(cipher.doFinal(Base64.decode(parts[1],Base64.NO_WRAP)),java.nio.charset.StandardCharsets.UTF_8);}
    private void saveConfig(String value)throws Exception{Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.ENCRYPT_MODE,key());String encrypted=Base64.encodeToString(cipher.getIV(),Base64.NO_WRAP)+":"+Base64.encodeToString(cipher.doFinal(value.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Base64.NO_WRAP);if(!getSharedPreferences("local-vault",MODE_PRIVATE).edit().putString("config",encrypted).commit())throw new IOException("保存配置失败");}
    private void eventError(Exception e){events.add("ERROR:"+e.getMessage());}
    @Override protected void onActivityResult(int request,int result,Intent intent){super.onActivityResult(request,result,intent);if(result!=RESULT_OK||intent==null)return;
        new Thread(()->{try{
            if(request==20){Uri uri=intent.getData();if(uri==null)return;try(InputStream in=new FileInputStream(exportSource);OutputStream out=getContentResolver().openOutputStream(uri,"wt")){if(out==null)throw new IOException("无法写入文件");copy(in,out,150*1024*1024);}events.add("{\"kind\":20}");return;}
            android.content.ClipData clip=intent.getClipData();JSONArray files=new JSONArray();int count=clip==null?1:clip.getItemCount();if(count>12)throw new IOException("一次最多选择12张照片");for(int index=0;index<count;index++){Uri uri=clip==null?intent.getData():clip.getItemAt(index).getUri();if(uri==null)continue;String name="file";try(Cursor cursor=getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){if(cursor!=null&&cursor.moveToFirst())name=cursor.getString(0);}File dir=new File(getFilesDir(),"incoming");dir.mkdirs();File file=File.createTempFile("import-",".data",dir);try(InputStream in=getContentResolver().openInputStream(uri);OutputStream out=new FileOutputStream(file)){if(in==null)throw new IOException("无法读取文件");copy(in,out,request==4?100*1024*1024:30*1024*1024);}files.put(new JSONObject().put("name",name).put("path",file.getAbsolutePath()));}
            JSONObject event=new JSONObject().put("kind",request).put("files",files);if(files.length()>0){event.put("name",files.getJSONObject(0).getString("name")).put("path",files.getJSONObject(0).getString("path"));}events.add(event.toString());
        }catch(Exception e){eventError(e);}},"easygot-documents").start();
    }
    private static void copy(InputStream in,OutputStream out,int max)throws IOException{byte[] buffer=new byte[8192];int n,total=0;while((n=in.read(buffer))!=-1){total+=n;if(total>max)throw new IOException("文件超过大小限制");out.write(buffer,0,n);}}
    private String renderPdf(String path)throws Exception{
        File source=new File(path);if(!source.getCanonicalPath().startsWith(getFilesDir().getCanonicalPath()+"/"))throw new IOException("非法PDF路径");JSONArray pages=new JSONArray();File dir=new File(getFilesDir(),"rendered");dir.mkdirs();
        try(android.os.ParcelFileDescriptor fd=android.os.ParcelFileDescriptor.open(source,android.os.ParcelFileDescriptor.MODE_READ_ONLY);android.graphics.pdf.PdfRenderer pdf=new android.graphics.pdf.PdfRenderer(fd)){
            if(pdf.getPageCount()>120)throw new IOException("视觉识别最多120页，请拆分文件");
            for(int i=0;i<pdf.getPageCount();i++)try(android.graphics.pdf.PdfRenderer.Page page=pdf.openPage(i)){int w=1400,h=page.getHeight()*w/page.getWidth();if(h>4000){h=4000;w=page.getWidth()*h/page.getHeight();}android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(w,h,android.graphics.Bitmap.Config.ARGB_8888);bitmap.eraseColor(android.graphics.Color.WHITE);page.render(bitmap,null,null,android.graphics.pdf.PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY);File file=File.createTempFile("page-",".jpg",dir);try(OutputStream out=new FileOutputStream(file)){bitmap.compress(android.graphics.Bitmap.CompressFormat.JPEG,85,out);}bitmap.recycle();pages.put(new JSONObject().put("path",file.getAbsolutePath()).put("locator","第 "+(i+1)+" 页（视觉识别）"));}
        }return pages.toString();
    }
}
