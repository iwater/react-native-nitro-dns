import { type HybridObject } from 'react-native-nitro-modules'

export interface NitroResolver extends HybridObject<{ ios: 'swift', android: 'kotlin' }> {
    /**
     * Cancel all outstanding DNS queries made by this resolver.
     */
    cancel(): void

    getServers(): string
    setServers(servers: string): void

    resolveipv4(hostname: string): Promise<string>
    resolveipv6(hostname: string): Promise<string>
    resolveMx(hostname: string): Promise<string>
    resolveTxt(hostname: string): Promise<string>
    resolveBase64Txt(hostname: string): Promise<string> // Not standard Node, but useful? Node just has resolveTxt
    resolveCname(hostname: string): Promise<string>
    resolveNs(hostname: string): Promise<string>
    resolveSoa(hostname: string): Promise<string>
    resolveSrv(hostname: string): Promise<string>
    resolveCaa(hostname: string): Promise<string>
    resolveNaptr(hostname: string): Promise<string>
    resolvePtr(hostname: string): Promise<string>
    resolveTlsa(hostname: string): Promise<string>
    resolveAny(hostname: string): Promise<string>
    reverse(ip: string): Promise<string>

    setLocalAddress(v4: string, v6: string): void
    clearCache(): void
}

export enum CachePolicy {
    FollowDnsTtl = 0,
    Bypass = 1,
    StaleWhileRevalidate = 2,
    StaleIfError = 3,
}

export interface NitroDns extends HybridObject<{ ios: 'swift', android: 'kotlin' }> {
    // Factory
    createResolver(config?: string): NitroResolver

    // Global Config
    getServers(): string
    setServers(servers: string): void

    // Lookup
    lookup(hostname: string, family: number): string

    // Resolution (Global/Default)
    resolve4(hostname: string): Promise<string>
    resolve6(hostname: string): Promise<string>
    resolveMx(hostname: string): Promise<string>
    resolveTxt(hostname: string): Promise<string>
    resolveCname(hostname: string): Promise<string>
    resolveNs(hostname: string): Promise<string>
    resolveSoa(hostname: string): Promise<string>
    resolveSrv(hostname: string): Promise<string>
    resolveCaa(hostname: string): Promise<string>
    resolveNaptr(hostname: string): Promise<string>
    resolvePtr(hostname: string): Promise<string>
    resolveTlsa(hostname: string): Promise<string>
    resolveAny(hostname: string): Promise<string>
    reverse(ip: string): Promise<string>
    lookupService(address: string, port: number): Promise<string>

    /**
     * Set whether app-wide native network requests (fetch, etc.) should be intercepted by this module's DNS.
     * Supported on Android (OkHttp Hook) and iOS (NSURLProtocol).
     */
    setNativeInterceptionEnabled(enabled: boolean): void
    setVerbose(enabled: boolean): void
    clearCache(): void
    setCacheSize(size: number): void
    setCachePolicy(policy: CachePolicy, staleTtl: number): void
}
