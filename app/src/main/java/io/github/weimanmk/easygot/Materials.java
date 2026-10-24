package io.github.weimanmk.easygot;

import android.content.*;
import android.net.Uri;
import android.database.Cursor;
import android.provider.OpenableColumns;
import android.graphics.*;
import android.graphics.pdf.PdfRenderer;
import android.os.ParcelFileDescriptor;
import android.util.Xml;
import org.xmlpull.v1.XmlPullParser;
import org.json.*;
import java.io.*;
import java.nio.charset.*;
import java.util.*;
import java.util.zip.*;
import java.util.function.Consumer;
import com.tom_roush.pdfbox.pdmodel.PDDocument;
import com.tom_roush.pdfbox.text.PDFTextStripper;

public final class Materials {
    private final Context ctx;private final Store store;
    public Materials(Context ctx,Store store){this.ctx=ctx;this.store=store;}
    public String name(Uri uri) {
        try(Cursor c=ctx.getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){if(c!=null&&c.moveToFirst())return c.getString(0);}catch(Exception ignored){}
        return "学习资料";
    }
    public byte[] bytes(Uri uri) throws IOException {try(InputStream in=ctx.getContentResolver().openInputStream(uri)){return Store.read(in,30*1024*1024);}}
    public JSONObject importFile(Uri uri,String role,String strict,Consumer<String> progress) throws Exception {
        String name=name(uri),ext=name.toLowerCase(Locale.ROOT);byte[] data=bytes(uri);
        String asset=store.asset(data);JSONArray sections=new JSONArray(),images=new JSONArray();
        if(ext.endsWith(".pdf")) {
            com.tom_roush.pdfbox.android.PDFBoxResourceLoader.init(ctx);
            try(PDDocument doc=PDDocument.load(data)) {
                if(doc.isEncrypted())throw new IOException("暂不支持加密 PDF，请先解密");
                int count=doc.getNumberOfPages();if(count>500)throw new IOException("首版单份 PDF 最多 500 页，请拆分");
                PDFTextStripper stripper=new PDFTextStripper();stripper.setSortByPosition(true);
                for(int page=1;page<=count;page++){progress.accept("提取 PDF："+page+"/"+count);stripper.setStartPage(page);stripper.setEndPage(page);sections.put(section("第 "+page+" 页",stripper.getText(doc)));}
            }
        } else if(ext.endsWith(".docx")) {
            Map<String,byte[]> files=new LinkedHashMap<>();int total=0;
            try(ZipInputStream zip=new ZipInputStream(new ByteArrayInputStream(data))) {ZipEntry entry;while((entry=zip.getNextEntry())!=null){String path=entry.getName();if(path.equals("word/document.xml")||path.startsWith("word/media/")){byte[] b=Store.read(zip,25*1024*1024);total+=b.length;if(total>60*1024*1024)throw new IOException("Word 解压内容过大");files.put(path,b);}zip.closeEntry();}}
            byte[] xml=files.get("word/document.xml");if(xml==null)throw new IOException("Word 文档缺少正文");
            XmlPullParser parser=Xml.newPullParser();parser.setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES,true);parser.setInput(new ByteArrayInputStream(xml),"UTF-8");
            StringBuilder paragraph=new StringBuilder();int index=0;boolean inText=false;
            for(int e=parser.getEventType();e!=XmlPullParser.END_DOCUMENT;e=parser.next()) {
                if(e==XmlPullParser.START_TAG){String tag=parser.getName();if(tag.equals("t"))inText=true;if(tag.equals("tab"))paragraph.append('\t');if(tag.equals("br"))paragraph.append('\n');}
                if(e==XmlPullParser.TEXT&&inText)paragraph.append(parser.getText());
                if(e==XmlPullParser.END_TAG){if(parser.getName().equals("t"))inText=false;if(parser.getName().equals("p")){index++;sections.put(section("第 "+index+" 段",paragraph.toString()));paragraph.setLength(0);}}
            }
            for(Map.Entry<String,byte[]> f:files.entrySet())if(f.getKey().startsWith("word/media/")) {
                BitmapFactory.Options opts=new BitmapFactory.Options();opts.inJustDecodeBounds=true;BitmapFactory.decodeByteArray(f.getValue(),0,f.getValue().length,opts);
                if(opts.outWidth>0)images.put(new JSONObject().put("asset",store.asset(f.getValue())).put("locator","内嵌图片 "+f.getKey().substring(11)));
                else sections.put(section("未支持的内嵌素材",f.getKey()+"：暂无法识别此图片格式，请转换 PNG/JPEG 后重新导入。"));
            }
        } else if(ext.endsWith(".txt")||ext.endsWith(".md")) {
            String text=decode(data);String[] paragraphs=text.split("\\R\\s*\\R");int n=1;
            for(String p:paragraphs){for(int at=0;at<Math.max(1,p.length());at+=6000)sections.put(section("第 "+n+++" 段",p.substring(at,Math.min(p.length(),at+6000))));}
        } else throw new IOException("支持 PDF、TXT、Markdown 和 DOCX；旧版 DOC 请先另存 DOCX");
        JSONObject material=new JSONObject().put("id",UUID.randomUUID().toString()).put("title",name).put("role",role).put("strictRules",strict).put("asset",asset).put("format",ext.endsWith(".pdf")?"pdf":ext.endsWith(".docx")?"docx":"text").put("sections",sections).put("images",images).put("importedAt",System.currentTimeMillis());
        store.save("material",material.getString("id"),material);return material;
    }
    private static JSONObject section(String locator,String text) throws JSONException {return new JSONObject().put("locator",locator).put("text",text);}
    private static String decode(byte[] bytes) throws Exception {
        if(bytes.length>=2&&bytes[0]==(byte)0xFF&&bytes[1]==(byte)0xFE)return new String(bytes,2,bytes.length-2,StandardCharsets.UTF_16LE);
        try{return StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).decode(java.nio.ByteBuffer.wrap(bytes)).toString().replace("\uFEFF","");}
        catch(CharacterCodingException e){return new String(bytes,Charset.forName("GB18030"));}
    }
    public JSONObject recognizeImages(JSONObject original,Ai ai,Consumer<String> progress) throws Exception {
        JSONObject m=new JSONObject(original.toString());JSONArray images=m.getJSONArray("images"),sections=m.getJSONArray("sections");
        if(m.optString("format").equals("pdf")) {
            try(ParcelFileDescriptor fd=ParcelFileDescriptor.open(store.assetFile(m.getString("asset")),ParcelFileDescriptor.MODE_READ_ONLY);PdfRenderer pdf=new PdfRenderer(fd)) {
                if(pdf.getPageCount()>120)throw new IOException("图片识别单次最多处理 120 页，请拆分教材；文字提取仍可离线使用");
                if(!m.optBoolean("pagesRendered")) {
                    for(int i=0;i<pdf.getPageCount();i++){progress.accept("准备 PDF 页面："+(i+1));try(PdfRenderer.Page page=pdf.openPage(i)){
                        int width=1500,height=Math.max(1,page.getHeight()*width/page.getWidth());if(height>4000){height=4000;width=Math.max(1,page.getWidth()*height/page.getHeight());}
                        Bitmap bitmap=Bitmap.createBitmap(width,height,Bitmap.Config.ARGB_8888);bitmap.eraseColor(Color.WHITE);page.render(bitmap,null,null,PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY);ByteArrayOutputStream out=new ByteArrayOutputStream();bitmap.compress(Bitmap.CompressFormat.JPEG,85,out);bitmap.recycle();
                        images.put(new JSONObject().put("asset",store.asset(out.toByteArray())).put("locator","第 "+(i+1)+" 页（视觉识别）"));
                    }}
                    m.put("pagesRendered",true);store.save("material",m.getString("id"),m);
                }
            }
        }
        for(int i=0;i<images.length();i++) {
            JSONObject image=images.getJSONObject(i);if(image.optBoolean("recognized"))continue;
            progress.accept("AI 识别图片 "+(i+1)+"/"+images.length());String text=ai.recognize(image.getString("asset"));
            sections.put(section(image.getString("locator"),text));image.put("recognized",true);
            // Save each completed page so a network failure can resume without repeating paid requests.
            store.save("material",m.getString("id"),m);
        }
        return m;
    }
}
