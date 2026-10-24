package io.github.weimanmk.easygot;

import android.util.Base64;
import android.util.Xml;
import org.xmlpull.v1.XmlPullParser;
import org.json.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.function.Consumer;

/** Append-only per-sync logs avoid last-writer-wins manifest races. No remote deletes. */
public final class WebDav {
    private static final Object LOCK=new Object();
    private final Store store;private final String base,auth;
    public WebDav(Store store,Vault vault) throws Exception {
        this.store=store;String root=vault.get("davUrl").trim();if(root.isBlank())throw new IOException("请先配置 WebDAV");
        base=Net.https(root).replaceAll("/+$","")+"/EasyGot/";
        auth="Basic "+Base64.encodeToString((vault.get("davUser")+":"+vault.get("davPassword")).getBytes(StandardCharsets.UTF_8),Base64.NO_WRAP);
    }
    private Net.Response request(String method,String relative,byte[] body,Map<String,String> headers,int max) throws Exception {return Net.request(method,base+relative,auth,body,headers,max);}
    private void directory(String relative) throws Exception {int code=request("MKCOL",relative,null,Map.of(),1024*1024).status();if(code!=201&&code!=405)throw new IOException(Net.error(code)+"；请检查 WebDAV 根目录是否已存在");}
    public void test() throws Exception {directory("");directory("logs/");directory("assets/");listLogs();}
    public String sync(Consumer<String> progress) throws Exception {
        synchronized(LOCK) {
            test();List<String> names=listLogs();int loaded=0;
            for(String name:names){if(store.received(name))continue;progress.accept("下载变更 "+(++loaded)+"："+name);
                Net.Response res=request("GET","logs/"+name,null,Map.of(),50*1024*1024);if(res.status()!=200)throw new IOException(Net.error(res.status()));
                JSONObject log=new JSONObject(new String(res.bytes(),StandardCharsets.UTF_8));if(log.getInt("syncVersion")!=1)throw new IOException("云端同步格式版本不受支持");
                JSONArray revisions=log.getJSONArray("revisions");Set<String> refs=Store.assetRefs(revisions);
                for(String hash:refs)if(!store.assetFile(hash).exists()) {
                    progress.accept("下载素材 "+hash.substring(0,8));Net.Response file=request("GET","assets/"+hash,null,Map.of(),60*1024*1024);
                    if(file.status()!=200)throw new IOException(Net.error(file.status()));if(!Store.sha(file.bytes()).equals(hash))throw new IOException("素材校验失败，未导入关联记录");store.asset(file.bytes());
                }
                store.ingest(revisions);store.markReceived(name);
            }
            List<JSONObject> all=store.revisions(null);JSONArray revisions=new JSONArray();for(JSONObject r:all)revisions.put(r);
            for(String hash:Store.assetRefs(revisions)) {
                Net.Response head=request("HEAD","assets/"+hash,null,Map.of(),1024);
                if(head.status()==200)continue;
                if(head.status()!=404)throw new IOException(Net.error(head.status()));
                progress.accept("上传素材 "+hash.substring(0,8));byte[] data;try(InputStream in=new FileInputStream(store.assetFile(hash))){data=Store.read(in,60*1024*1024);}
                putImmutable("assets/"+hash,data,"application/octet-stream");
            }
            if(!all.isEmpty()) {
                String name=UUID.randomUUID()+".json";JSONObject log=new JSONObject().put("syncVersion",1).put("revisions",revisions);
                byte[] data=log.toString().getBytes(StandardCharsets.UTF_8);if(data.length>50*1024*1024)throw new IOException("变更历史超过 50MB，首版暂不支持分片；本机数据仍保留");
                progress.accept("上传学习记录…");putImmutable("logs/"+name,data,"application/json; charset=utf-8");store.markReceived(name);
            }
            return "同步完成：读取 "+loaded+" 个变更包，待处理冲突 "+store.conflicts().size()+" 项";
        }
    }
    private void putImmutable(String path,byte[] bytes,String type) throws Exception {
        int code=request("PUT",path,bytes,Map.of("Content-Type",type,"If-None-Match","*"),1024*1024).status();
        if(code!=200&&code!=201&&code!=204&&code!=412)throw new IOException(Net.error(code));
        if(code==412){Net.Response existing=request("GET",path,null,Map.of(),60*1024*1024);if(existing.status()!=200||!Store.sha(existing.bytes()).equals(Store.sha(bytes)))throw new IOException("云端同名文件不一致，已中止同步");}
    }
    private List<String> listLogs() throws Exception {
        byte[] xml="<?xml version=\"1.0\"?><d:propfind xmlns:d=\"DAV:\"><d:prop><d:resourcetype/></d:prop></d:propfind>".getBytes(StandardCharsets.UTF_8);
        Net.Response response=request("PROPFIND","logs/",xml,Map.of("Depth","1","Content-Type","application/xml"),8*1024*1024);
        if(response.status()!=207)throw new IOException(Net.error(response.status())+"；服务器需支持 PROPFIND Depth:1");
        XmlPullParser parser=Xml.newPullParser();parser.setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES,true);parser.setInput(new ByteArrayInputStream(response.bytes()),"UTF-8");List<String> names=new ArrayList<>();
        for(int e=parser.getEventType();e!=XmlPullParser.END_DOCUMENT;e=parser.next())if(e==XmlPullParser.START_TAG&&parser.getName().equals("href")) {
            String href=parser.nextText();String path=new URI(href).getPath();if(path==null)continue;String name=path.substring(path.lastIndexOf('/')+1);
            if(name.matches("[a-fA-F0-9-]{36}\\.json"))names.add(name);
        }
        Collections.sort(names);return names;
    }
}
