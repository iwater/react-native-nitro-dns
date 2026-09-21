/**
 * LoopRef —— Node 的「在途请求 / 长寿命 handle 顶住事件循环」语义到
 * **宿主能力**的桥。
 *
 * 背景：Node 里在途的异步请求本身就会让 libuv 的 loop 不退出（active request），
 * 本库的宿主未必有这个语义：
 *   - qjs-node / qjs-run（quickjs-jsi）：EventLoop 在「无待办」时收泵，
 *     提供 globalThis.__qjsHandleRef 供库登记存活；
 *   - React Native：App 生命周期归 OS，JS 空转不退出 —— 没有这个全局，
 *     本类所有方法是 no-op，行为与今天完全一致。
 *
 * 与 react-native-nitro-net 的同名文件**逐字相同**（约定各库各持一份、独立演进，
 * 不抽共享包）。两种用法：
 *   - handle 型：每 handle 一个实例，acquire/release 幂等；
 *   - 请求型：发起时 acquire、settle 时 release（成功失败都要，见 index.ts）。
 */

interface HostHandleRef {
  acquire(label: string): number;
  release(token: number): void;
}

// 模块加载时探测一次。宿主能力不会在进程中途出现/消失。
let host: HostHandleRef | undefined =
  (globalThis as { __qjsHandleRef?: HostHandleRef }).__qjsHandleRef;

/** 仅测试用：替换/清除探测到的宿主能力。 */
export function __setHostHandleRefForTest(h: HostHandleRef | undefined): void {
  host = h;
}

export class LoopRef {
  private token: number | null = null;

  constructor(private readonly label: string) {}

  /** 已持有时 no-op。 */
  acquire(): void {
    if (host && this.token === null) {
      this.token = host.acquire(this.label);
    }
  }

  /** 未持有时 no-op。 */
  release(): void {
    if (host && this.token !== null) {
      host.release(this.token);
      this.token = null;
    }
  }

  /** 当前是否正顶着 loop（诊断/测试用）。 */
  get refed(): boolean {
    return this.token !== null;
  }
}
