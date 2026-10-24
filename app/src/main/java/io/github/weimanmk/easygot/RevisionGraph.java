package io.github.weimanmk.easygot;

import org.json.*;
import java.util.*;

/** Immutable revisions. A version disappears from heads only when explicitly parented. */
public final class RevisionGraph {
    public static List<JSONObject> heads(List<JSONObject> revisions) throws JSONException {
        Set<String> parents=new HashSet<>();
        for(JSONObject r:revisions) { JSONArray p=r.getJSONArray("parents"); for(int i=0;i<p.length();i++) parents.add(p.getString(i)); }
        List<JSONObject> heads=new ArrayList<>();
        for(JSONObject r:revisions) if(!parents.contains(r.getString("version"))) heads.add(r);
        heads.sort(Comparator.comparing(r->r.optString("version")));
        return heads;
    }
    public static void validate(List<JSONObject> revisions) throws JSONException {
        Map<String,JSONObject> map=new HashMap<>();
        for(JSONObject r:revisions) {
            Domain.required(r,"version"); Domain.required(r,"entityId");
            if(!Set.of("question","material","attempt","note","session","plan").contains(r.getString("kind"))) throw new JSONException("未知记录类型");
            r.getJSONObject("payload"); r.getJSONArray("parents");
            if(map.put(r.getString("version"),r)!=null) throw new JSONException("重复版本");
        }
        Map<String,Integer> indegrees=new HashMap<>(); Map<String,List<String>> children=new HashMap<>();
        for(JSONObject r:revisions) {
            String id=r.getString("version"); JSONArray p=r.getJSONArray("parents"); Set<String> seen=new HashSet<>();
            indegrees.put(id,p.length());
            for(int i=0;i<p.length();i++) {
                String parent=p.getString(i); JSONObject prior=map.get(parent);
                if(!seen.add(parent)||prior==null||!prior.getString("entityId").equals(r.getString("entityId"))||!prior.getString("kind").equals(r.getString("kind"))) throw new JSONException("缺少父版本或跨记录引用");
                children.computeIfAbsent(parent,k->new ArrayList<>()).add(id);
            }
        }
        Deque<String> queue=new ArrayDeque<>(); indegrees.forEach((k,v)->{if(v==0)queue.add(k);}); int visited=0;
        while(!queue.isEmpty()) {String id=queue.remove();visited++;for(String child:children.getOrDefault(id,List.of())) {int n=indegrees.merge(child,-1,Integer::sum);if(n==0)queue.add(child);} }
        if(visited!=map.size()) throw new JSONException("版本图存在循环");
    }
}
