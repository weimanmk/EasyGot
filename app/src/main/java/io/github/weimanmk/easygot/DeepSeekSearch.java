package io.github.weimanmk.easygot;

import org.json.*;

/** DeepSeek Search/Fetch API, verified against official docs on 2026-10-24. */
public final class DeepSeekSearch {
    private final String base,key;
    public DeepSeekSearch(String base,String key){this.base=base.replaceAll("/+$","");this.key=key;}
    public static JSONObject searchRequest(String query,int count,boolean content) throws JSONException {
        String q=query.trim();int size=q.codePointCount(0,q.length());
        if(size==0||size>128)throw new JSONException("搜索词需要 1–128 个 Unicode 字符");
        if(count<1||count>50)throw new JSONException("搜索条数需要在 1–50 之间");
        return new JSONObject().put("query",q).put("count",count).put("need_content",content);
    }
    public JSONArray search(String query) throws Exception {
        JSONObject result=Net.post(base+"/v1/tools/search",key,searchRequest(query,5,true));
        return result.getJSONArray("results");
    }
    public JSONObject fetch(String url) throws Exception {
        java.net.URI u=new java.net.URI(url);
        if(u.getHost()==null||u.getUserInfo()!=null||url.length()>8192||!("http".equals(u.getScheme())||"https".equals(u.getScheme())))throw new JSONException("抓取目标地址不合法");
        return Net.post(base+"/v1/tools/fetch",key,new JSONObject().put("url",url));
    }
}
