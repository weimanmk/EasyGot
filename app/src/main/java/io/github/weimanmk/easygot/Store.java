package io.github.weimanmk.easygot;

import android.content.*;
import android.database.Cursor;
import android.database.sqlite.*;
import org.json.*;
import java.util.*;
import java.io.*;
import java.security.*;

public final class Store extends SQLiteOpenHelper {
    public final File assets;
    public Store(Context ctx) { super(ctx,"easygot.db",null,1); assets=new File(ctx.getFilesDir(),"assets"); if(!assets.exists()&&!assets.mkdirs()) throw new IllegalStateException("无法创建文件目录"); }
    @Override public void onCreate(SQLiteDatabase db) {
        db.execSQL("CREATE TABLE revisions(version TEXT PRIMARY KEY, entityId TEXT NOT NULL, kind TEXT NOT NULL, json TEXT NOT NULL)");
        db.execSQL("CREATE INDEX entities ON revisions(entityId)");
        db.execSQL("CREATE TABLE received(name TEXT PRIMARY KEY)");
    }
    @Override public void onUpgrade(SQLiteDatabase db,int old,int next) { throw new IllegalStateException("缺少数据库迁移"); }
    public synchronized List<JSONObject> revisions(String id) throws JSONException {
        List<JSONObject> out=new ArrayList<>();
        try(Cursor c=getReadableDatabase().query("revisions",new String[]{"json"},id==null?null:"entityId=?",id==null?null:new String[]{id},null,null,"rowid")) { while(c.moveToNext()) out.add(new JSONObject(c.getString(0))); }
        return out;
    }
    public synchronized List<JSONObject> heads(String id) throws JSONException { return RevisionGraph.heads(revisions(id)); }
    public synchronized JSONObject get(String id) throws JSONException {
        List<JSONObject> heads=heads(id); return heads.isEmpty()?null:heads.get(0);
    }
    public synchronized List<JSONObject> list(String kind) throws JSONException {
        List<JSONObject> out=new ArrayList<>(); List<String> ids=new ArrayList<>();
        try(Cursor c=getReadableDatabase().rawQuery("SELECT DISTINCT entityId FROM revisions WHERE kind=? ORDER BY rowid DESC",new String[]{kind})) {while(c.moveToNext()) ids.add(c.getString(0));}
        for(String id:ids) {JSONObject r=get(id);if(r!=null&&!r.optBoolean("deleted")) out.add(r.getJSONObject("payload"));}
        return out;
    }
    public synchronized List<String> conflicts() throws JSONException {
        List<String> result=new ArrayList<>(); Set<String> ids=new LinkedHashSet<>();
        for(JSONObject r:revisions(null)) ids.add(r.getString("entityId"));
        for(String id:ids) if(heads(id).size()>1) result.add(id);
        return result;
    }
    public synchronized JSONObject save(String kind,String id,JSONObject payload) throws JSONException { return save(kind,id,payload,false); }
    public synchronized JSONObject save(String kind,String id,JSONObject payload,boolean deleted) throws JSONException {
        if(heads(id).size()>1)throw new JSONException("此记录存在同步冲突，请先在设置中处理");
        return resolve(kind,id,payload,deleted);
    }
    public synchronized JSONObject resolve(String kind,String id,JSONObject payload,boolean deleted) throws JSONException {
        JSONArray parents=new JSONArray();for(JSONObject r:heads(id)) parents.put(r.getString("version"));
        JSONObject rev=new JSONObject().put("version",UUID.randomUUID().toString()).put("entityId",id).put("kind",kind).put("parents",parents).put("modifiedAt",System.currentTimeMillis()).put("deleted",deleted).put("payload",new JSONObject(payload.toString()));
        insert(rev); return rev;
    }
    private void insert(JSONObject r) throws JSONException {
        ContentValues v=new ContentValues(); v.put("version",r.getString("version"));v.put("entityId",r.getString("entityId"));v.put("kind",r.getString("kind"));v.put("json",r.toString());
        getWritableDatabase().insertOrThrow("revisions",null,v);
    }
    public synchronized void saveBatch(String kind,JSONArray payloads) throws JSONException {
        SQLiteDatabase db=getWritableDatabase();db.beginTransaction();
        try {for(int i=0;i<payloads.length();i++) {JSONObject p=payloads.getJSONObject(i);save(kind,p.getString("id"),p);}db.setTransactionSuccessful();}finally{db.endTransaction();}
    }
    public synchronized void ingest(JSONArray incoming) throws JSONException {
        Map<String,JSONObject> all=new LinkedHashMap<>();for(JSONObject r:revisions(null)) all.put(r.getString("version"),r);
        List<JSONObject> additions=new ArrayList<>();
        for(int i=0;i<incoming.length();i++) {
            JSONObject r=incoming.getJSONObject(i);String version=r.getString("version");
            if(all.containsKey(version)) { if(!sameJson(all.get(version),r)) throw new JSONException("同版本内容不一致"); }
            else {all.put(version,r);additions.add(r);}
        }
        RevisionGraph.validate(new ArrayList<>(all.values()));
        for(JSONObject r:additions) {
            JSONObject payload=r.getJSONObject("payload");
            if(!r.getString("entityId").equals(payload.getString("id"))) throw new JSONException("记录 id 不一致");
            if(r.getString("kind").equals("question")) Domain.question(payload);
        }
        SQLiteDatabase db=getWritableDatabase();db.beginTransaction();try{for(JSONObject r:additions)insert(r);db.setTransactionSuccessful();}finally{db.endTransaction();}
    }
    private static boolean sameJson(Object a,Object b) throws JSONException {
        if(a instanceof JSONObject && b instanceof JSONObject) {JSONObject x=(JSONObject)a,y=(JSONObject)b;if(x.length()!=y.length())return false;Iterator<String> keys=x.keys();while(keys.hasNext()){String k=keys.next();if(!y.has(k)||!sameJson(x.get(k),y.get(k)))return false;}return true;}
        if(a instanceof JSONArray && b instanceof JSONArray) {JSONArray x=(JSONArray)a,y=(JSONArray)b;if(x.length()!=y.length())return false;for(int i=0;i<x.length();i++)if(!sameJson(x.get(i),y.get(i)))return false;return true;}
        return Objects.equals(a,b);
    }
    public synchronized boolean received(String name) {try(Cursor c=getReadableDatabase().rawQuery("SELECT name FROM received WHERE name=?",new String[]{name})){return c.moveToFirst();}}
    public synchronized void markReceived(String name) {ContentValues v=new ContentValues();v.put("name",name);getWritableDatabase().insertWithOnConflict("received",null,v,SQLiteDatabase.CONFLICT_IGNORE);}
    public String asset(byte[] bytes) throws Exception {
        String hash=sha(bytes);File dest=new File(assets,hash);
        if(!dest.exists()) {File tmp=File.createTempFile("asset-",".part",assets);try{try(FileOutputStream out=new FileOutputStream(tmp)){out.write(bytes);out.getFD().sync();}if(!tmp.renameTo(dest)&&!dest.exists())throw new IOException("保存素材失败");}finally{tmp.delete();}}
        return hash;
    }
    public File assetFile(String hash) { if(!hash.matches("[a-f0-9]{64}"))throw new IllegalArgumentException("非法素材标识");return new File(assets,hash); }
    public static String sha(byte[] bytes) throws Exception {byte[] h=MessageDigest.getInstance("SHA-256").digest(bytes);StringBuilder b=new StringBuilder();for(byte x:h)b.append(String.format(Locale.ROOT,"%02x",x));return b.toString();}
    public static byte[] read(InputStream in,int max) throws IOException {
        if(in==null)throw new IOException("无法读取文件");ByteArrayOutputStream out=new ByteArrayOutputStream();byte[] buf=new byte[8192];int n;
        while((n=in.read(buf))!=-1){if(out.size()+n>max)throw new IOException("文件或响应超过限制 "+max/1024/1024+" MB");out.write(buf,0,n);}return out.toByteArray();
    }
    public static Set<String> assetRefs(Object object) throws JSONException {
        Set<String> hashes=new HashSet<>();
        if(object instanceof JSONObject){JSONObject o=(JSONObject)object;Iterator<String> it=o.keys();while(it.hasNext()){String key=it.next();Object value=o.get(key);if((key.equals("asset")||key.equals("image"))&&value instanceof String&&((String)value).matches("[a-f0-9]{64}"))hashes.add((String)value);else hashes.addAll(assetRefs(value));}}
        else if(object instanceof JSONArray){JSONArray a=(JSONArray)object;for(int i=0;i<a.length();i++)hashes.addAll(assetRefs(a.get(i)));}
        return hashes;
    }
}
