package io.github.weimanmk.easygot;
import android.print.*;
import android.os.*;
import java.io.*;
/** Android system print/save-as-PDF sheet for an already generated immutable paper. */
final class PdfPrintAdapter extends PrintDocumentAdapter {
    private final File file;PdfPrintAdapter(File file){this.file=file;}
    @Override public void onLayout(PrintAttributes oldAttributes,PrintAttributes newAttributes,CancellationSignal signal,LayoutResultCallback callback,Bundle extras){if(signal.isCanceled()){callback.onLayoutCancelled();return;}callback.onLayoutFinished(new PrintDocumentInfo.Builder(file.getName()).setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT).build(),true);}
    @Override public void onWrite(PageRange[] pages,ParcelFileDescriptor destination,CancellationSignal signal,WriteResultCallback callback){try(InputStream in=new FileInputStream(file);OutputStream out=new FileOutputStream(destination.getFileDescriptor())){byte[] bytes=new byte[8192];int n;while((n=in.read(bytes))!=-1){if(signal.isCanceled()){callback.onWriteCancelled();return;}out.write(bytes,0,n);}callback.onWriteFinished(new PageRange[]{PageRange.ALL_PAGES});}catch(IOException error){callback.onWriteFailed("无法输出试卷，请重试");}}
}
