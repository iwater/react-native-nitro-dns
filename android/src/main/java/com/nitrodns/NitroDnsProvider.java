package com.nitrodns;

import android.util.Log;
import androidx.annotation.NonNull;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.ArrayList;
import java.util.List;
import okhttp3.Dns;

public class NitroDnsProvider implements Dns {
    private static final String TAG = "NitroDns";
    private static boolean isEnabled = false;

    public static void setEnabled(boolean enabled) {
        Log.d(TAG, "NitroDnsProvider.setEnabled: " + enabled);
        isEnabled = enabled;
    }

    public static boolean isEnabled() {
        return isEnabled;
    }

    @NonNull
    @Override
    public List<InetAddress> lookup(@NonNull String hostname) throws UnknownHostException {
        Log.d(TAG, "NitroDnsProvider.lookup: " + hostname + " (enabled: " + isEnabled + ")");
        if (isEnabled) {
            try {
                String json = NitroDns.resolve(hostname);
                Log.d(TAG, "NitroDnsProvider.lookup: resolved JSON for " + hostname + ": " + json);
                if (json != null && !json.equals("[]")) {
                    org.json.JSONArray array = new org.json.JSONArray(json);
                    List<InetAddress> addresses = new ArrayList<>();
                    for (int i = 0; i < array.length(); i++) {
                        addresses.add(InetAddress.getByName(array.getString(i)));
                    }
                    return addresses;
                }
            } catch (Exception e) {
                Log.e(TAG, "NitroDnsProvider.lookup: Error resolving " + hostname, e);
                // Fallback to system on error
            }
        }
        return Dns.SYSTEM.lookup(hostname);
    }
}
