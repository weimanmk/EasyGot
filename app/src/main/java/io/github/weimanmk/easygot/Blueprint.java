package io.github.weimanmk.easygot;

import org.json.*;
import java.util.*;

/** Deterministic question counts for syllabus weights, independently validated after AI generation. */
public final class Blueprint {
    public static void validate(JSONObject spec)throws JSONException {
        int count=spec.getInt("count"),sum=0;JSONObject types=spec.getJSONObject("typeCounts");Iterator<String> keys=types.keys();
        while(keys.hasNext()){String k=keys.next();int n=types.getInt(k);if(!Domain.TYPES.contains(k)||n<0)throw new JSONException("题型或题量非法");sum+=n;}
        if(sum!=count)throw new JSONException("各题型题数之和必须等于题量");
        JSONObject weights=spec.getJSONObject("weights");List<String> topics=new ArrayList<>();Iterator<String> it=weights.keys();double total=0;
        while(it.hasNext()){String key=it.next();double w=weights.getDouble(key);if(!Double.isFinite(w)||w<=0||key.isBlank())throw new JSONException("知识点权重必须是正数");topics.add(key);total+=w;}
        JSONObject targets=new JSONObject();if(!topics.isEmpty()){
            Collections.sort(topics);int assigned=0;Map<String,Double> remainders=new HashMap<>();
            for(String topic:topics){double exact=count*weights.getDouble(topic)/total;int n=(int)Math.floor(exact);targets.put(topic,n);assigned+=n;remainders.put(topic,exact-n);}
            topics.sort((a,b)->{int c=Double.compare(remainders.get(b),remainders.get(a));return c!=0?c:a.compareTo(b);});
            for(int i=0;i<count-assigned;i++){String topic=topics.get(i);targets.put(topic,targets.getInt(topic)+1);}
        }
        spec.put("knowledgePointCounts",targets);
    }
    public static void verify(JSONObject spec,JSONArray questions)throws JSONException {
        if(questions.length()!=spec.getInt("count"))throw new JSONException("生成题数不符合要求");
        Map<String,Integer> types=new HashMap<>(),topics=new HashMap<>();
        for(int i=0;i<questions.length();i++){
            JSONObject q=questions.getJSONObject(i);types.merge(q.getString("type"),1,Integer::sum);
            if(q.optInt("difficulty")!=spec.getInt("difficulty"))throw new JSONException("AI 题目难度与要求不符");
            if(!spec.optString("chapter").isBlank()&&!spec.getString("chapter").equals(q.optString("chapter")))throw new JSONException("AI 题目章节与要求不符");
            JSONArray points=q.optJSONArray("knowledgePoints");if(points!=null&&points.length()>0)topics.merge(points.getString(0),1,Integer::sum);
        }
        JSONObject requested=spec.getJSONObject("typeCounts");for(String type:Domain.TYPES)if(types.getOrDefault(type,0)!=requested.optInt(type))throw new JSONException("AI 生成题型配比不符合要求");
        JSONObject counts=spec.getJSONObject("knowledgePointCounts");Iterator<String> keys=counts.keys();while(keys.hasNext()){String key=keys.next();if(topics.getOrDefault(key,0)!=counts.getInt(key))throw new JSONException("AI 生成知识点配比不符合考纲权重，请重试");}
    }
}
