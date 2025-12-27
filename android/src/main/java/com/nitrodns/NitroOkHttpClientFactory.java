package com.nitrodns;

import android.util.Log;
import com.facebook.react.modules.network.OkHttpClientFactory;
import com.facebook.react.modules.network.OkHttpClientProvider;
import okhttp3.OkHttpClient;

public class NitroOkHttpClientFactory implements OkHttpClientFactory {
    private static final String TAG = "NitroDns";

    @Override
    public OkHttpClient createNewNetworkModuleClient() {
        Log.d(TAG, "NitroOkHttpClientFactory: Creating new client with NitroDnsProvider");
        return OkHttpClientProvider.createClientBuilder()
                .dns(new NitroDnsProvider())
                .build();
    }
}
