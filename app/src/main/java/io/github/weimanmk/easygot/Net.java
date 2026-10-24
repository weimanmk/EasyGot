package io.github.weimanmk.easygot;

import org.json.*;
import java.net.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

public final class Net {
    public static String https(String url) throws IOException {
        try {URI u=new URI(url);if(!"https".equalsIgnoreCase(u.getScheme())||u.getHost()==null||u.getUserInfo()!=null||u.getFragment()!=null)throw new Exception();return u.toASCIIString();}
        catch(Exception e){throw new IOException("地址必须是 HTTPS，且不能包含用户名、密码或片段");}
    }
    public record Response(int status,byte[] bytes) {}
    public static Response request(String method,String url,String auth,byte[] body,Map<String,String> headers,int max) throws Exception {
        okhttp3.OkHttpClient client=new okhttp3.OkHttpClient.Builder()
            .connectTimeout(20,java.util.concurrent.TimeUnit.SECONDS)
            .readTimeout(120,java.util.concurrent.TimeUnit.SECONDS)
            .callTimeout(180,java.util.concurrent.TimeUnit.SECONDS)
            .followRedirects(false).followSslRedirects(false).build();
        okhttp3.Request.Builder request=new okhttp3.Request.Builder().url(https(url)).header("Accept","application/json");
        if(!auth.isEmpty())request.header("Authorization",auth);
        for(Map.Entry<String,String> h:headers.entrySet())request.header(h.getKey(),h.getValue());
        okhttp3.RequestBody payload=body==null?null:okhttp3.RequestBody.create(body,okhttp3.MediaType.parse(headers.getOrDefault("Content-Type","application/octet-stream")));
        request.method(method,payload);
        try(okhttp3.Response response=client.newCall(request.build()).execute()) {
            byte[] bytes=new byte[0];
            if(response.body()!=null)try(InputStream in=response.body().byteStream()){bytes=Store.read(in,max);}
            return new Response(response.code(),bytes);
        } finally {client.connectionPool().evictAll();client.dispatcher().executorService().shutdown();}
    }

    public static JSONObject post(String url,String key,JSONObject body) throws Exception {
        if(key.isBlank())throw new IOException("请先配置 API Key");
        Response r=request("POST",url,"Bearer "+key,body.toString().getBytes(StandardCharsets.UTF_8),Map.of("Content-Type","application/json"),8*1024*1024);
        if(r.status<200||r.status>=300)throw new IOException(error(r.status));
        return new JSONObject(new String(r.bytes,StandardCharsets.UTF_8));
    }
    public static String error(int code){return switch(code){case 400->"请求参数不兼容（HTTP 400），请检查模型和接口配置";case 401->"认证失败（HTTP 401），请检查 Key 或 WebDAV 账号密码";case 402->"服务余额不足（HTTP 402）";case 403->"没有访问权限（HTTP 403）";case 404->"接口或文件不存在（HTTP 404）";case 413->"请求过大（HTTP 413），请缩小图片或资料范围";case 429->"请求频率超限（HTTP 429），请稍后手动重试";default->"网络请求失败（HTTP "+code+"），请检查服务状态；重定向不会自动携带凭据";};}
}
