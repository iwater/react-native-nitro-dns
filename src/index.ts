import { NitroModules } from 'react-native-nitro-modules'
import {
    type NitroDns,
    type NitroResolver,
    CachePolicy
} from './NitroDns.nitro'
import { LoopRef } from './loopRef'

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

// Constants（与 Node 对齐：getaddrinfo 的 AI_* 实际值；Node 没有 HINTS 导出）
export const ADDRCONFIG = 1024;
export const V4MAPPED = 2048;
export const ALL = 256;

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
// Node 兼容的参数校验（lookupService 用）
// isIP 正则逐字移植自 node v22 lib/internal/net.js；校验顺序、错误类型与
// code 对齐 lib/dns.js 的 lookupService 与 lib/internal/validators.js 的
// validatePort（均为同步抛出）。
// --------------------------------------------------------------------------
const _v4Seg = '(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])';
const _v4Str = `(?:${_v4Seg}\\.){3}${_v4Seg}`;
const _IPv4Reg = new RegExp(`^${_v4Str}$`);
const _v6Seg = '(?:[0-9a-fA-F]{1,4})';
const _IPv6Reg = new RegExp('^(?:' +
    `(?:${_v6Seg}:){7}(?:${_v6Seg}|:)|` +
    `(?:${_v6Seg}:){6}(?:${_v4Str}|:${_v6Seg}|:)|` +
    `(?:${_v6Seg}:){5}(?::${_v4Str}|(?::${_v6Seg}){1,2}|:)|` +
    `(?:${_v6Seg}:){4}(?:(?::${_v6Seg}){0,1}:${_v4Str}|(?::${_v6Seg}){1,3}|:)|` +
    `(?:${_v6Seg}:){3}(?:(?::${_v6Seg}){0,2}:${_v4Str}|(?::${_v6Seg}){1,4}|:)|` +
    `(?:${_v6Seg}:){2}(?:(?::${_v6Seg}){0,3}:${_v4Str}|(?::${_v6Seg}){1,5}|:)|` +
    `(?:${_v6Seg}:){1}(?:(?::${_v6Seg}){0,4}:${_v4Str}|(?::${_v6Seg}){1,6}|:)|` +
    `(?::(?:(?::${_v6Seg}){0,5}:${_v4Str}|(?::${_v6Seg}){1,7}|:))` +
')(?:%[0-9a-zA-Z-.:]{1,})?$');

function _isIP(s: any): number {
    if (typeof s !== 'string') return 0;
    if (_IPv4Reg.test(s)) return 4;
    if (_IPv6Reg.test(s)) return 6;
    return 0;
}

function _inspect(v: any): string {
    return typeof v === 'string' ? `'${v}'` : String(v);
}

function _throwInvalidArgValue(name: string, value: any): never {
    const err: any = new TypeError(`The argument '${name}' is invalid. Received ${_inspect(value)}`);
    err.code = 'ERR_INVALID_ARG_VALUE';
    throw err;
}

// node:internal/validators validatePort 的逐字移植（lookupService 处恒为
// name='Port', allowZero=true，故去掉 allowZero 参数与 `port === 0` 分支）
function _validatePort(port: any): void {
    if ((typeof port !== 'number' && typeof port !== 'string') ||
        (typeof port === 'string' && port.trim().length === 0) ||
        +port !== (+port >>> 0) ||
        Number(port) > 0xFFFF) {
        const err: any = new RangeError(
            `Port should be >= 0 and < 65536. Received type ${typeof port} (${_inspect(port)}).`);
        err.code = 'ERR_SOCKET_BAD_PORT';
        throw err;
    }
}

// callback 与 promises 两个入口共享的输入校验（同步抛，与 Node 一致）
function _validateLookupServiceInput(address: any, port: any): void {
    if (_isIP(address) === 0) {
        _throwInvalidArgValue('address', address);
    }
    _validatePort(port);
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
    // 校验与 node:dns.lookupService 同序、同步抛出（lib/dns.js，Node v22）
    if (arguments.length !== 3) {
        const err: any = new TypeError(
            'The "address", "port", and "callback" arguments must be specified');
        err.code = 'ERR_MISSING_ARGS';
        throw err;
    }
    _validateLookupServiceInput(address, port);
    if (typeof callback !== 'function') {
        const err: any = new TypeError(
            `Callback must be a function. Received ${_inspect(callback)}`);
        err.code = 'ERR_INVALID_CALLBACK';
        throw err;
    }
    port = +port;
    const done = trackRequest();
    NitroDnsModule.lookupService(address, port).then(json => {
        try {
            const res = JSON.parse(json);
            callback(null, res.hostname, res.service);
        } finally { done(); }
    }).catch((e: any) => {
        try {
            const err = createError('ENOTFOUND', 'lookupService failed');
            callback(err, "", "");
        } finally { done(); }
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

// --------------------------------------------------------------------------
// 在途请求的 handle-ref（请求型）
// --------------------------------------------------------------------------
//
// 本模块的原生请求是真异步：`resolve*` / `lookupService` 返回的 Promise 要等
// Rust 侧查完才 settle。宿主（quickjs-jsi 系）的 EventLoop 看不见这个在途请求，
// 于是「脚本最后一个动作是 dns.resolve4(host, cb)」时 loop 会**先收泵** →
// 回调静默丢失（实测：进程 exit 0，cb 根本不跑）。
// Node 里没有这个问题：在途请求本身就是 loop 的活跃项。
//
// ⚠️ 实测澄清（2026-09-22）：`dns.lookup()` **不在此列** —— 它的原生方法是
// **同步**返回 JSON 的（`NitroDnsModule.lookup()` 直接给字符串，没有 Promise），
// 回调在调用栈里就跑完了，不需要登记。
//
// 用法：发起前 `const done = trackRequest()`，settle 后 `done()`（成功/失败都要，
// 用 try/finally 包住回调，避免回调自己抛错时漏一次 release → 进程挂死）。
function trackRequest(): () => void {
    const ref = new LoopRef('dns.request');
    ref.acquire();
    return () => ref.release();
}

// Resolve Helpers
function resolveHelper<T>(
    promiseCall: (host: string) => Promise<string>,
    hostname: string,
    callback?: ResolveCallback,
    transform?: (data: any) => T
) {
    if (!callback) return;
    const done = trackRequest();
    promiseCall(hostname).then(json => {
        try {
            const data = JSON.parse(json);
            callback(null, transform ? transform(data) : data);
        } finally { done(); }
    }).catch((err: any) => {
        try { callback(err, null); } finally { done(); }
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
    const done = trackRequest();
    promiseCall(hostname).then(json => {
        try {
            const data = JSON.parse(json); // [{address, ttl, family}]
            if (options && options.ttl) {
                const result = data.map((d: any) => ({ address: d.address, ttl: d.ttl }));
                callback(null, result);
            } else {
                const result = data.map((d: any) => d.address);
                callback(null, result);
            }
        } finally { done(); }
    }).catch((err: any) => {
        try { callback(err, null); } finally { done(); }
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
        const done = trackRequest();
        p.then(s => {
            try {
                let res = JSON.parse(s);
                if ((type === 'A' || type === 'AAAA') && Array.isArray(res) && res.length > 0 && typeof res[0] === 'object') {
                    res = res.map((r: any) => r.address);
                }
                cb(null, res)
            } finally { done(); }
        }).catch((e: any) => {
            try { cb(e, null); } finally { done(); }
        });
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
// ⚠️ 这两个是**第二个入口面**（promises 面），与回调面（resolveHelper /
// resolveHelperIp / resolveDispatch / lookupService）走的是同一批原生请求，
// 所以同样必须登记 handle-ref —— 否则在途请求顶不住 loop、promise 永不 settle、
// 进程静默 exit 0（这条线曾经漏掉，守卫见 quickjs-jsi 的 R29 第 ③ 段）。
// 用 try/finally：回调（这里是 await）自己抛错时也不能漏一次 release。
async function handlePromise(fn: (h: string) => Promise<string>, host: string) {
    const done = trackRequest();
    try {
        return JSON.parse(await fn(host));
    } finally { done(); }
}
async function handlePromiseIp(fn: (h: string) => Promise<string>, host: string, options?: any) {
    const done = trackRequest();
    try {
        const json = await fn(host);
        const data = JSON.parse(json);
        if (options && options.ttl) {
            return data.map((d: any) => ({ address: d.address, ttl: d.ttl }));
        }
        return data.map((d: any) => d.address);
    } finally { done(); }
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
    lookupService: (address: string, port: number) => {
        // 与 Node 的 promises.lookupService 一致：输入校验**同步抛出**（在创建
        // Promise 之前），见 node:internal/dns/promises 的 lookupService
        _validateLookupServiceInput(address, port);
        return new Promise((res, rej) => lookupService(address, +port, (err, hostname, service) => err ? rej(err) : res({ hostname, service } as any)));
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
