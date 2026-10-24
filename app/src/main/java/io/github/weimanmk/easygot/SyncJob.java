package io.github.weimanmk.easygot;

import android.app.job.*;
import android.content.*;
import java.util.concurrent.*;

public final class SyncJob extends JobService {
    private final ExecutorService executor=Executors.newSingleThreadExecutor();private Future<?> work;
    public static void schedule(Context ctx,boolean enabled){JobScheduler scheduler=(JobScheduler)ctx.getSystemService(Context.JOB_SCHEDULER_SERVICE);if(!enabled){scheduler.cancel(1001);return;}scheduler.schedule(new JobInfo.Builder(1001,new ComponentName(ctx,SyncJob.class)).setRequiredNetworkType(JobInfo.NETWORK_TYPE_UNMETERED).setPeriodic(6*60*60*1000L).build());}
    @Override public boolean onStartJob(JobParameters params){work=executor.submit(()->{try(Store store=new Store(this)){String result=new WebDav(store,new Vault(this)).sync(s->{});getSharedPreferences("status",MODE_PRIVATE).edit().putString("sync",result).apply();jobFinished(params,false);}catch(Exception e){getSharedPreferences("status",MODE_PRIVATE).edit().putString("sync","自动同步未完成："+e.getMessage()).apply();jobFinished(params,false);}});return true;}
    @Override public boolean onStopJob(JobParameters params){if(work!=null)work.cancel(true);return true;}
    @Override public void onDestroy(){executor.shutdownNow();super.onDestroy();}
}
