package com.nitrodns;

import java.util.Arrays;
import java.util.List;

public class NitroDns {
    static {
        System.loadLibrary("RNDns");
    }

    /**
     * Synchronously resolves a hostname using the Nitro DNS (Rust) logic.
     * @param hostname The hostname to resolve.
     * @return An array of IP address strings.
     */
    public static native String resolve(String hostname);
}
