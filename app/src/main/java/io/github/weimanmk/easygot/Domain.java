package io.github.weimanmk.easygot;

import org.json.*;
import java.util.*;

/** Platform-independent validation and grading rules; all imported/AI data crosses this boundary. */
public final class Domain {
    private Domain() {}
    public static final Set<String> TYPES = Set.of("single", "multiple", "boolean", "short", "essay", "calculation");
    public static String required(JSONObject o, String key) throws JSONException {
        Object v = o.get(key);
        if (!(v instanceof String) || ((String)v).trim().isEmpty()) throw new JSONException(key + " 必须是非空字符串");
        return (String)v;
    }
    public static void question(JSONObject q) throws JSONException {
        required(q,"id"); required(q,"stem");
        String type = required(q,"type");
        if (!TYPES.contains(type)) throw new JSONException("不支持的题型 " + type);
        double max = q.getDouble("maxScore");
        if (!Double.isFinite(max) || max <= 0 || max > 10000) throw new JSONException("maxScore 必须在 (0,10000] 内");
        if (q.optString("origin").equals("ai") && q.optJSONArray("sources") == null) throw new JSONException("AI 题必须提供 sources（无依据时为空数组）");
        if (type.equals("single") || type.equals("multiple")) {
            JSONArray options = q.getJSONArray("options"), answer = q.getJSONArray("answer");
            if (options.length() < 2 || options.length() > 12) throw new JSONException("选择题需要 2–12 个选项");
            Set<String> keys = new HashSet<>();
            for (int i=0;i<options.length();i++) {
                JSONObject option=options.getJSONObject(i);
                if (!keys.add(required(option,"id"))) throw new JSONException("选项 id 重复");
                required(option,"text");
            }
            if (answer.length()==0 || (type.equals("single") && answer.length()!=1)) throw new JSONException("答案数量错误");
            Set<String> answers = new HashSet<>();
            for (int i=0;i<answer.length();i++) if (!keys.contains(answer.getString(i)) || !answers.add(answer.getString(i))) throw new JSONException("答案必须为不重复的选项 id");
        } else if (type.equals("boolean")) {
            if (!(q.get("answer") instanceof Boolean)) throw new JSONException("判断题 answer 必须是 true/false");
        } else required(q,"referenceAnswer");
        if (q.has("knowledgePoints")) strings(q.getJSONArray("knowledgePoints"));
        if (q.has("materialIds")) strings(q.getJSONArray("materialIds"));
        if (q.has("sources")) {
            JSONArray sources=q.getJSONArray("sources");
            for(int i=0;i<sources.length();i++) {
                JSONObject s=sources.getJSONObject(i);
                required(s,"title"); required(s,"locator");
                if(!s.has("materialId") && !s.has("url")) throw new JSONException("来源必须包含 materialId 或 url");
            }
        }
    }
    public static void strings(JSONArray a) throws JSONException { for(int i=0;i<a.length();i++) if(!(a.get(i) instanceof String)) throw new JSONException("数组只能包含字符串"); }
    public static JSONArray bank(JSONObject bank) throws JSONException {
        if (bank.getInt("schemaVersion")!=1) throw new JSONException("不支持的 schemaVersion");
        required(bank,"title"); JSONArray qs=bank.getJSONArray("questions");
        if(qs.length()>20000) throw new JSONException("单次最多导入 20000 题");
        Set<String> ids=new HashSet<>();
        for(int i=0;i<qs.length();i++) { JSONObject q=qs.getJSONObject(i); question(q); if(!ids.add(q.getString("id"))) throw new JSONException("题目 id 重复"); }
        return qs;
    }
    public static boolean subjective(JSONObject q) { return Set.of("short","essay","calculation").contains(q.optString("type")); }
    public static double objectiveScore(JSONObject q, Object answer) throws JSONException {
        if(subjective(q)) throw new JSONException("主观题需要 AI 或人工评分");
        boolean correct;
        if(q.getString("type").equals("boolean")) correct = answer instanceof Boolean && answer.equals(q.get("answer"));
        else {
            Set<String> expected=new HashSet<>(), actual=new HashSet<>();
            JSONArray a=q.getJSONArray("answer"); for(int i=0;i<a.length();i++) expected.add(a.getString(i));
            if(!(answer instanceof JSONArray)) return 0;
            JSONArray b=(JSONArray)answer; for(int i=0;i<b.length();i++) actual.add(b.getString(i));
            correct=expected.equals(actual) && actual.size()==b.length();
        }
        return correct?q.getDouble("maxScore"):0;
    }
    public static JSONObject parseObject(String text) throws JSONException {
        String s=text.trim();
        if(s.startsWith("```")) { int start=s.indexOf('\n'),end=s.lastIndexOf("```"); if(start>=0&&end>start) s=s.substring(start+1,end).trim(); }
        return new JSONObject(s);
    }
    public static JSONObject checkedGrade(JSONObject grade, double max) throws JSONException {
        double score=grade.getDouble("score");
        if(!Double.isFinite(score)||score<0||score>max) throw new JSONException("AI 分数超出合法范围，未保存");
        required(grade,"feedback"); required(grade,"explanation");
        if(!grade.has("citations")) grade.put("citations",new JSONArray());
        return grade;
    }
    public static String clip(String s,int limit) { return s.length()>limit?s.substring(0,limit)+"…":s; }
    public static String typeLabel(String t) { return switch(t) { case "single"->"单选";case "multiple"->"多选";case "boolean"->"判断";case "short"->"简答";case "essay"->"论述";case "calculation"->"计算";default->t;}; }
}
