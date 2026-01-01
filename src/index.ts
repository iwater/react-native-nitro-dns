import { NitroModules } from 'react-native-nitro-modules'
import {
    type NitroDns,
    type NitroResolver,
    CachePolicy
} from './NitroDns.nitro'

const NitroDnsModule = NitroModules.createHybridObject<NitroDns>('NitroDns')

export { CachePolicy };

// Types
export interface LookupOptions {
    family?: number;
    hints?: number;
    all?: boolean;
    verbatim?: boolean;
    order?: 'ipv4first' | 'ipv6first' | 'verbatim';
}

export interface LookupAddress {
    address: string;
    family: number;
}

export type LookupServiceCallback = (err: Error | null, hostname: string, service: string) => void;
export type LookupCallback = (err: Error | null, address: string | LookupAddress[], family?: number) => void;
export type ResolveCallback = (err: Error | null, records: any) => void;

// Constants
export const ADDRCONFIG = 1;
export const V4MAPPED = 2;
export const ALL = 4;

export const HINTS = {
    ADDRCONFIG: 1024,
    V4MAPPED: 2048,
    ALL: 256,
};

let defaultResultOrder: 'ipv4first' | 'ipv6first' | 'verbatim' = 'verbatim';

/**
 * Sets the default value of the `order` parameter used in `dns.lookup()`.
 */
export function setDefaultResultOrder(order: 'ipv4first' | 'ipv6first' | 'verbatim'): void {
    defaultResultOrder = order;
}

/**
 * Returns the default value of `order` parameter used in `dns.lookup()`.
 */
export function getDefaultResultOrder(): string {
    return defaultResultOrder;
}

// Error helpers
function createError(code: string, message: string, hostname?: string) {
    const err: any = new Error(`${code} ${message} ${hostname || ''}`);
    err.code = code;
    err.hostname = hostname;
    return err;
}

// --------------------------------------------------------------------------
// Lookup
// --------------------------------------------------------------------------
export function lookup(hostname: string, options?: number | LookupOptions | LookupCallback, callback?: LookupCallback): void {
    let cb: any;
    let opts: LookupOptions = {};

    if (typeof options === 'function') {
        cb = options;
    } else if (typeof options === 'number') {
        opts = { family: options };
        if (typeof callback === 'function') cb = callback;
    } else if (typeof options === 'object') {
        opts = options;
        if (typeof callback === 'function') cb = callback;
    } else {
        if (typeof callback === 'function') cb = callback;
    }

    const family = opts.family || 0;

    try {
        const resJson = NitroDnsModule.lookup(hostname, family);
        let ips: string[] = JSON.parse(resJson);

        if (!cb) return;

        if (ips.length === 0) {
            cb(createError('ENOTFOUND', 'not found', hostname), "", 4);
            return;
        }

        // Sorting logic: options.order > options.verbatim > defaultResultOrder
        let order = opts.order;
        if (order === undefined) {
            if (opts.verbatim === false) {
                order = 'ipv4first';
            } else if (opts.verbatim === true) {
                order = 'verbatim';
            } else {
                order = defaultResultOrder;
            }
        }

        if (order === 'ipv4first' || order === 'ipv6first') {
            ips = ips.sort((a, b) => {
                const aIsV4 = !a.includes(':');
                const bIsV4 = !b.includes(':');
                if (aIsV4 === bIsV4) return 0;
                if (order === 'ipv4first') {
                    return aIsV4 ? -1 : 1;
                } else {
                    return aIsV4 ? 1 : -1;
                }
            });
        }

        if (opts.all) {
            const addresses: LookupAddress[] = ips.map(ip => ({
                address: ip,
                family: ip.includes(':') ? 6 : 4
            }));
            cb(null, addresses);
        } else {
            const ip = ips[0];
            cb(null, ip, ip.includes(':') ? 6 : 4);
        }

    } catch (e: any) {
        if (cb) cb(createError('EAI_AGAIN', e.message || 'lookup failed', hostname), "", 4);
    }
}


// --------------------------------------------------------------------------
// Lookup Service
// --------------------------------------------------------------------------
export function lookupService(address: string, port: number, callback: (err: Error | null, hostname: string, service: string) => void): void {
    NitroDnsModule.lookupService(address, port).then(json => {
        const res = JSON.parse(json);
        callback(null, res.hostname, res.service);
    }).catch((e: any) => {
        const err = createError('ENOTFOUND', 'lookupService failed');
        callback(err, "", "");
    });
}


// --------------------------------------------------------------------------
// Resolver Class
// --------------------------------------------------------------------------
export class Resolver {
    private _nitroResolver: NitroResolver;

    constructor(options?: { servers?: string[], timeout?: number, tries?: number, maxTimeout?: number, cacheSize?: number }) {
        let configJson: string | undefined = undefined;
        if (options) {
            const config: any = {};
            if (options.servers) config.servers = options.servers;
            if (options.timeout !== undefined) config.timeout = options.timeout; // -1 or >0
            if (options.tries !== undefined) config.attempts = options.tries;
            if (options.maxTimeout !== undefined) config.maxTimeout = options.maxTimeout;
            if (options.cacheSize !== undefined) config.cacheSize = options.cacheSize;
            if (Object.keys(config).length > 0) {
                configJson = JSON.stringify(config);
            }
        }
        this._nitroResolver = NitroDnsModule.createResolver(configJson);
    }

    cancel(): void {
        this._nitroResolver.cancel();
    }

    getServers(): string[] {
        return JSON.parse(this._nitroResolver.getServers());
    }

    setServers(servers: string[]): void {
        this._nitroResolver.setServers(JSON.stringify(servers));
    }

    setLocalAddress(ipv4?: string, ipv6?: string): void {
        this._nitroResolver.setLocalAddress(ipv4 || '0.0.0.0', ipv6 || '::0');
    }

    clearCache(): void {
        this._nitroResolver.clearCache();
    }

    resolve(hostname: string, rrtype: string = 'A', callback: ResolveCallback): void {
        resolveDispatch(this._nitroResolver, hostname, rrtype, callback);
    }

    resolve4(hostname: string, options?: any, callback?: ResolveCallback): void {
        let cb = callback || (typeof options === 'function' ? options : undefined);
        let opts = (typeof options === 'object') ? options : {};
        resolveHelperIp(this._nitroResolver.resolveipv4.bind(this._nitroResolver), hostname, opts, cb);
    }

    resolve6(hostname: string, options?: any, callback?: ResolveCallback): void {
        let cb = callback || (typeof options === 'function' ? options : undefined);
        let opts = (typeof options === 'object') ? options : {};
        resolveHelperIp(this._nitroResolver.resolveipv6.bind(this._nitroResolver), hostname, opts, cb);
    }

    resolveAny(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveAny.bind(this._nitroResolver), hostname, callback); }
    resolveCname(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveCname.bind(this._nitroResolver), hostname, callback); }
    resolveMx(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveMx.bind(this._nitroResolver), hostname, callback); }
    resolveNaptr(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveNaptr.bind(this._nitroResolver), hostname, callback); }
    resolveNs(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveNs.bind(this._nitroResolver), hostname, callback); }
    resolvePtr(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolvePtr.bind(this._nitroResolver), hostname, callback); }
    resolveSoa(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveSoa.bind(this._nitroResolver), hostname, callback); }
    resolveSrv(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveSrv.bind(this._nitroResolver), hostname, callback); }
    resolveCaa(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveCaa.bind(this._nitroResolver), hostname, callback); }
    resolveTxt(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveTxt.bind(this._nitroResolver), hostname, callback); }
    resolveTlsa(hostname: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.resolveTlsa.bind(this._nitroResolver), hostname, callback); }

    reverse(ip: string, callback: ResolveCallback) { resolveHelper(this._nitroResolver.reverse.bind(this._nitroResolver), ip, callback); }
}

// --------------------------------------------------------------------------
// Global Functions
// --------------------------------------------------------------------------

export function getServers(): string[] {
    return JSON.parse(NitroDnsModule.getServers());
}

export function setServers(servers: string[]): void {
    NitroDnsModule.setServers(JSON.stringify(servers));
}

// Resolve Helpers
function resolveHelper<T>(
    promiseCall: (host: string) => Promise<string>,
    hostname: string,
    callback?: ResolveCallback,
    transform?: (data: any) => T
) {
    if (!callback) return;
    promiseCall(hostname).then(json => {
        const data = JSON.parse(json);
        callback(null, transform ? transform(data) : data);
    }).catch((err: any) => {
        callback(err, null);
    });
}

// Specialized helper for IPs with TTL support
function resolveHelperIp(
    promiseCall: (host: string) => Promise<string>,
    hostname: string,
    options: { ttl?: boolean },
    callback?: ResolveCallback
) {
    if (!callback) return;
    promiseCall(hostname).then(json => {
        const data = JSON.parse(json); // [{address, ttl, family}]
        if (options && options.ttl) {
            const result = data.map((d: any) => ({ address: d.address, ttl: d.ttl }));
            callback(null, result);
        } else {
            const result = data.map((d: any) => d.address);
            callback(null, result);
        }
    }).catch((err: any) => {
        callback(err, null);
    });
}


function resolveDispatch(resolver: any, hostname: string, rrtype: string | ResolveCallback, callback?: ResolveCallback) {
    let type = 'A';
    let cb: any = callback;
    if (typeof rrtype === 'function') {
        cb = rrtype;
    } else if (typeof rrtype === 'string') {
        type = rrtype;
    }

    const r = (resolver as any);
    let p: Promise<string>;

    switch (type) {
        case 'A': p = r.resolve4 ? r.resolve4(hostname) : r.resolveipv4(hostname); break;
        case 'AAAA': p = r.resolve6 ? r.resolve6(hostname) : r.resolveipv6(hostname); break;
        case 'ANY': p = r.resolveAny(hostname); break;
        case 'CNAME': p = r.resolveCname(hostname); break;
        case 'MX': p = r.resolveMx(hostname); break;
        case 'NAPTR': p = r.resolveNaptr(hostname); break;
        case 'NS': p = r.resolveNs(hostname); break;
        case 'PTR': p = r.resolvePtr(hostname); break;
        case 'SOA': p = r.resolveSoa(hostname); break;
        case 'SRV': p = r.resolveSrv(hostname); break;
        case 'TXT': p = r.resolveTxt(hostname); break;
        case 'CAA': p = r.resolveCaa(hostname); break;
        case 'TLSA': p = r.resolveTlsa(hostname); break;
        default:
            if (cb) cb(new Error(`Unknown rrtype: ${type}`), null);
            return;
    }

    if (cb) {
        p.then(s => {
            let res = JSON.parse(s);
            if ((type === 'A' || type === 'AAAA') && Array.isArray(res) && res.length > 0 && typeof res[0] === 'object') {
                res = res.map((r: any) => r.address);
            }
            cb(null, res)
        }).catch((e: any) => cb(e, null));
    }
}


export function resolve(hostname: string, rrtype: string = 'A', callback: ResolveCallback): void {
    resolveDispatch(NitroDnsModule, hostname, rrtype, callback);
}

export function resolve4(hostname: string, options?: any, callback?: ResolveCallback): void {
    let cb = callback || (typeof options === 'function' ? options : undefined);
    let opts = (typeof options === 'object') ? options : {};
    resolveHelperIp(NitroDnsModule.resolve4.bind(NitroDnsModule), hostname, opts, cb);
}

export function resolve6(hostname: string, options?: any, callback?: ResolveCallback): void {
    let cb = callback || (typeof options === 'function' ? options : undefined);
    let opts = (typeof options === 'object') ? options : {};
    resolveHelperIp(NitroDnsModule.resolve6.bind(NitroDnsModule), hostname, opts, cb);
}

export function resolveAny(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveAny.bind(NitroDnsModule), hostname, callback); }
export function resolveCname(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveCname.bind(NitroDnsModule), hostname, callback); }
export function resolveMx(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveMx.bind(NitroDnsModule), hostname, callback); }
export function resolveNaptr(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveNaptr.bind(NitroDnsModule), hostname, callback); }
export function resolveNs(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveNs.bind(NitroDnsModule), hostname, callback); }
export function resolvePtr(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolvePtr.bind(NitroDnsModule), hostname, callback); }
export function resolveSoa(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveSoa.bind(NitroDnsModule), hostname, callback); }
export function resolveSrv(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveSrv.bind(NitroDnsModule), hostname, callback); }
export function resolveCaa(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveCaa.bind(NitroDnsModule), hostname, callback); }
export function resolveTxt(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveTxt.bind(NitroDnsModule), hostname, callback); }
export function resolveTlsa(hostname: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.resolveTlsa.bind(NitroDnsModule), hostname, callback); }

export function reverse(ip: string, callback: ResolveCallback) { resolveHelper(NitroDnsModule.reverse.bind(NitroDnsModule), ip, callback); }

export function setNativeInterceptionEnabled(enabled: boolean): void {
    NitroDnsModule.setNativeInterceptionEnabled(enabled);
}

export function setVerbose(enabled: boolean): void {
    NitroDnsModule.setVerbose(enabled);
}

export function clearCache(): void {
    NitroDnsModule.clearCache();
}

export function setCacheSize(size: number): void {
    NitroDnsModule.setCacheSize(size);
}

export function setCachePolicy(policy: CachePolicy, staleTtl: number = 3600): void {
    NitroDnsModule.setCachePolicy(policy, staleTtl);
}

// --------------------------------------------------------------------------
// Promises API
// --------------------------------------------------------------------------
async function handlePromise(fn: (h: string) => Promise<string>, host: string) {
    return JSON.parse(await fn(host));
}
async function handlePromiseIp(fn: (h: string) => Promise<string>, host: string, options?: any) {
    const json = await fn(host);
    const data = JSON.parse(json);
    if (options && options.ttl) {
        return data.map((d: any) => ({ address: d.address, ttl: d.ttl }));
    }
    return data.map((d: any) => d.address);
}


export class ResolverPromises {
    private _r: Resolver;
    constructor(resolver: Resolver) { this._r = resolver; }

    getServers() { return this._r.getServers(); }
    setServers(s: string[]) { this._r.setServers(s); }
    cancel() { this._r.cancel(); }
    clearCache() { this._r.clearCache(); }

    resolve(hostname: string, rrtype: string = 'A') { return new Promise((res, rej) => this._r.resolve(hostname, rrtype, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolve4(hostname: string, options?: any) { return handlePromiseIp((h) => (this._r as any)._nitroResolver.resolveipv4(h), hostname, options); }
    resolve6(hostname: string, options?: any) { return handlePromiseIp((h) => (this._r as any)._nitroResolver.resolveipv6(h), hostname, options); }

    resolveAny(hostname: string) { return new Promise((res, rej) => this._r.resolveAny(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveCname(hostname: string) { return new Promise((res, rej) => this._r.resolveCname(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveMx(hostname: string) { return new Promise((res, rej) => this._r.resolveMx(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveNaptr(hostname: string) { return new Promise((res, rej) => this._r.resolveNaptr(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveNs(hostname: string) { return new Promise((res, rej) => this._r.resolveNs(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolvePtr(hostname: string) { return new Promise((res, rej) => this._r.resolvePtr(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveSoa(hostname: string) { return new Promise((res, rej) => this._r.resolveSoa(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveSrv(hostname: string) { return new Promise((res, rej) => this._r.resolveSrv(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveCaa(hostname: string) { return new Promise((res, rej) => this._r.resolveCaa(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveTxt(hostname: string) { return new Promise((res, rej) => this._r.resolveTxt(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    resolveTlsa(hostname: string) { return new Promise((res, rej) => this._r.resolveTlsa(hostname, (e: any, d: any) => e ? rej(e) : res(d))); }
    reverse(ip: string) { return new Promise((res, rej) => this._r.reverse(ip, (e: any, d: any) => e ? rej(e) : res(d))); }
}


export const promises = {
    getServers,
    setServers,
    lookup: async (hostname: string, options?: any) => {
        return new Promise((res, rej) => lookup(hostname, options, (err, address, family) => err ? rej(err) : res({ address, family } as any)));
    },
    lookupService: async (address: string, port: number) => {
        return new Promise((res, rej) => lookupService(address, port, (err, hostname, service) => err ? rej(err) : res({ hostname, service } as any)));
    },
    resolve: async (hostname: string, rrtype: string = 'A') => new Promise((res, rej) => resolve(hostname, rrtype, (e: any, d: any) => e ? rej(e) : res(d))),
    resolve4: async (hostname: string, options?: any) => new Promise((res, rej) => resolve4(hostname, options, (e: any, d: any) => e ? rej(e) : res(d))),
    resolve6: async (hostname: string, options?: any) => new Promise((res, rej) => resolve6(hostname, options, (e: any, d: any) => e ? rej(e) : res(d))),
    resolveAny: async (hostname: string) => handlePromise(NitroDnsModule.resolveAny.bind(NitroDnsModule), hostname),
    resolveCname: async (hostname: string) => handlePromise(NitroDnsModule.resolveCname.bind(NitroDnsModule), hostname),
    resolveMx: async (hostname: string) => handlePromise(NitroDnsModule.resolveMx.bind(NitroDnsModule), hostname),
    resolveNaptr: async (hostname: string) => handlePromise(NitroDnsModule.resolveNaptr.bind(NitroDnsModule), hostname),
    resolveNs: async (hostname: string) => handlePromise(NitroDnsModule.resolveNs.bind(NitroDnsModule), hostname),
    resolvePtr: async (hostname: string) => handlePromise(NitroDnsModule.resolvePtr.bind(NitroDnsModule), hostname),
    resolveSoa: async (hostname: string) => handlePromise(NitroDnsModule.resolveSoa.bind(NitroDnsModule), hostname),
    resolveSrv: async (hostname: string) => handlePromise(NitroDnsModule.resolveSrv.bind(NitroDnsModule), hostname),
    resolveCaa: async (hostname: string) => handlePromise(NitroDnsModule.resolveCaa.bind(NitroDnsModule), hostname),
    resolveTxt: async (hostname: string) => handlePromise(NitroDnsModule.resolveTxt.bind(NitroDnsModule), hostname),
    resolveTlsa: async (hostname: string) => handlePromise(NitroDnsModule.resolveTlsa.bind(NitroDnsModule), hostname),
    reverse: async (ip: string) => handlePromise(NitroDnsModule.reverse.bind(NitroDnsModule), ip),

    clearCache: async () => NitroDnsModule.clearCache(),
    setCacheSize: async (size: number) => NitroDnsModule.setCacheSize(size),
    setCachePolicy: async (policy: CachePolicy, staleTtl: number = 3600) => NitroDnsModule.setCachePolicy(policy, staleTtl),

    Resolver: ResolverPromises
};

export default {
    lookup,
    lookupService,
    getServers,
    setServers,
    resolve,
    resolve4,
    resolve6,
    resolveAny,
    resolveCname,
    resolveMx,
    resolveNaptr,
    resolveNs,
    resolvePtr,
    resolveSoa,
    resolveSrv,
    resolveCaa,
    resolveTxt,
    resolveTlsa,
    reverse,
    setNativeInterceptionEnabled,
    setVerbose,
    clearCache,
    setCacheSize,
    setCachePolicy,
    Resolver,
    promises,
    setDefaultResultOrder,
    getDefaultResultOrder,
    ADDRCONFIG,
    V4MAPPED,
    ALL
};
