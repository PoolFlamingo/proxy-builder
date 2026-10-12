/**
 * SmoothScroll for websites v1.4.10 (Balazs Galambosi)
 * http://www.smoothscroll.net/
 *
 * TypeScript singleton port — same module as ZodHub / base/web
 * (`/home/estigio/Projects/Nextjs/base/web/lib/SmoothScroll.ts`).
 * Wire via `SmoothScrollClient` + Apariencia → «Scroll suave».
 * NO Lenis: this animates native `scrollTop` on the overflowing ancestor.
 */

interface SmoothScrollOptions {
	frameRate: number;
	animationTime: number;
	stepSize: number;

	pulseAlgorithm: boolean;
	pulseScale: number;
	pulseNormalize: number;

	accelerationDelta: number;
	accelerationMax: number;

	keyboardSupport: boolean;
	arrowScroll: number;

	fixedBackground: boolean;
	excluded: string;
}

interface ScrollQueueItem {
	x: number;
	y: number;
	lastX: number;
	lastY: number;
	start: number;
}

interface Direction {
	x: number;
	y: number;
}

interface KeyCodes {
	left: number;
	up: number;
	right: number;
	down: number;
	spacebar: number;
	pageup: number;
	pagedown: number;
	end: number;
	home: number;
}

type ArrowKeys = Record<number, number>;

type WheelDeltaEvent = WheelEvent & {
	wheelDeltaX?: number;
	wheelDeltaY?: number;
	wheelDelta?: number;
};

type ParentWheelTarget = {
	wheel?: (event: WheelEvent) => boolean;
	keydown?: (event: KeyboardEvent) => boolean;
};

declare global {
	interface Element {
		uniqueID?: number;
		$scrollBehavior?: string | null;
	}

	interface Window {
		SmoothScrollOptions?: Partial<SmoothScrollOptions>;
		SmoothScroll?: typeof SmoothScroll;
	}
}

/**
 * ¿Puede este elemento desplazarse más en la dirección de la rueda?
 *
 * Pura, para poder probarla. `alto` es lo que se ve (`clientHeight`, o la
 * altura de la ventana si es la raíz del documento).
 */
export function puedeMoverseY(
	scrollTop: number,
	alto: number,
	scrollHeight: number,
	deltaY: number,
): boolean {
	if (!deltaY) return true;
	if (deltaY > 0) return scrollTop + alto < scrollHeight - 1;
	return scrollTop > 0;
}

export class SmoothScroll {
	private static instance: SmoothScroll | null = null;

	private readonly defaultOptions: SmoothScrollOptions = {
		frameRate: 150,
		animationTime: 400,
		stepSize: 100,

		pulseAlgorithm: true,
		pulseScale: 4,
		pulseNormalize: 1,

		accelerationDelta: 50,
		accelerationMax: 3,

		keyboardSupport: true,
		arrowScroll: 50,

		fixedBackground: true,
		excluded: "",
	};

	private options: SmoothScrollOptions;
	private isExcluded = false;
	private isFrame = false;
	private direction: Direction = { x: 0, y: 0 };
	private initDone = false;
	private root: HTMLElement;
	private activeElement: HTMLElement | null = null;
	private observer: MutationObserver | null = null;
	private refreshSize: (() => void) | null = null;
	private deltaBuffer: number[] = [];
	private deltaBufferTimer: number | null = null;
	private isMac = /^Mac/.test(navigator.platform);

	private readonly key: KeyCodes = {
		left: 37,
		up: 38,
		right: 39,
		down: 40,
		spacebar: 32,
		pageup: 33,
		pagedown: 34,
		end: 35,
		home: 36,
	};

	private readonly arrowKeys: ArrowKeys = { 37: 1, 38: 1, 39: 1, 40: 1 };

	private que: ScrollQueueItem[] = [];
	private pending = false;
	private lastScroll = Date.now();

	private uniqueIDCounter = 0;
	private cacheX: Record<number, Element> = {};
	private cacheY: Record<number, Element> = {};
	private clearCacheTimer: number | null = null;
	private smoothBehaviorForElement: Record<number, boolean | null> = {};

	private readonly userAgent = window.navigator.userAgent;
	private readonly isEdge = /Edge/.test(this.userAgent);
	private readonly isChrome = /chrome/i.test(this.userAgent) && !this.isEdge;
	private readonly isSafari = /safari/i.test(this.userAgent) && !this.isEdge;
	private readonly isMobile = /mobile/i.test(this.userAgent);
	private readonly isOldSafari =
		this.isSafari && (/Version\/8/i.test(this.userAgent) || /Version\/9/i.test(this.userAgent));
	// 🛑 JUDLEX is desktop-only (Tauri: WKWebView / WebView2 / WebKitGTK). The
	// upstream Chrome||Safari gate skips some WebKitGTK UAs and would leave
	// Linux without smooth scroll. Non-mobile is enough here.
	private readonly isEnabledForBrowser = !this.isMobile;

	private supportsPassive = false;
	private wheelOpt: boolean | { passive: boolean } = false;
	private wheelEvent = "";

	private readonly boundWheel: (event: WheelEvent) => boolean;
	private readonly boundKeydown: (event: KeyboardEvent) => boolean;
	private readonly boundMousedown: (event: MouseEvent) => void;
	private readonly boundInit: () => void;

	private constructor(optionsToSet?: Partial<SmoothScrollOptions>) {
		this.options = { ...this.defaultOptions };

		if (optionsToSet) {
			this.setOptions(optionsToSet);
		}

		this.root = document.compatMode.indexOf("CSS") >= 0 ? document.documentElement : document.body;

		this.boundWheel = this.wheel.bind(this);
		this.boundKeydown = this.keydown.bind(this);
		this.boundMousedown = this.mousedown.bind(this);
		this.boundInit = this.init.bind(this);

		this.detectPassiveSupport();
		this.setupWheelEvent();
		this.loadDeltaBuffer();

		if (this.wheelEvent && this.isEnabledForBrowser) {
			this.initializeEvents();
		}
	}

	public static getInstance(optionsToSet?: Partial<SmoothScrollOptions>): SmoothScroll {
		if (!SmoothScroll.instance) {
			SmoothScroll.instance = new SmoothScroll(optionsToSet);
		} else if (optionsToSet) {
			SmoothScroll.instance.setOptions(optionsToSet);
		}
		return SmoothScroll.instance;
	}

	public static destroy(): void {
		if (SmoothScroll.instance) {
			SmoothScroll.instance.cleanup();
			SmoothScroll.instance = null;
		}
	}

	public setOptions(optionsToSet: Partial<SmoothScrollOptions>): void {
		for (const key of Object.keys(optionsToSet) as (keyof SmoothScrollOptions)[]) {
			if (Object.hasOwn(this.defaultOptions, key)) {
				const value = optionsToSet[key];
				if (value !== undefined) {
					(this.options[key] as SmoothScrollOptions[typeof key]) =
						value as SmoothScrollOptions[typeof key];
				}
			}
		}
	}

	private detectPassiveSupport(): void {
		try {
			const opts = Object.defineProperty({}, "passive", {
				get: () => {
					this.supportsPassive = true;
				},
			});
			window.addEventListener("test", (() => {}) as EventListener, opts);
		} catch {
			// Passive events not supported
		}
	}

	private setupWheelEvent(): void {
		this.wheelOpt = this.supportsPassive ? { passive: false } : false;
		this.wheelEvent = "onwheel" in document.createElement("div") ? "wheel" : "mousewheel";
	}

	private loadDeltaBuffer(): void {
		if (typeof window !== "undefined" && window.localStorage && localStorage.SS_deltaBuffer) {
			try {
				this.deltaBuffer = String(localStorage.SS_deltaBuffer).split(",").map(Number);
			} catch {
				// Ignore corrupted buffer
			}
		}
	}

	private initializeEvents(): void {
		this.addEvent(this.wheelEvent, this.boundWheel as unknown as EventListener, this.wheelOpt);
		this.addEvent("mousedown", this.boundMousedown as unknown as EventListener);
		this.addEvent("load", this.boundInit as unknown as EventListener);
	}

	private initTest(): void {
		if (this.options.keyboardSupport) {
			this.addEvent("keydown", this.boundKeydown as unknown as EventListener);
		}
	}

	private init(): void {
		if (this.initDone || !document.body) return;

		this.initDone = true;

		const body = document.body;
		const html = document.documentElement;
		const windowHeight = window.innerHeight;
		const scrollHeight = body.scrollHeight;

		this.root = document.compatMode.indexOf("CSS") >= 0 ? html : body;
		this.activeElement = body;

		this.initTest();

		if (top !== self) {
			this.isFrame = true;
		} else if (
			this.isOldSafari &&
			scrollHeight > windowHeight &&
			(body.offsetHeight <= windowHeight || html.offsetHeight <= windowHeight)
		) {
			const fullPageElem = document.createElement("div");
			fullPageElem.style.cssText = `position:absolute; z-index:-10000; top:0; left:0; right:0; height:${this.root.scrollHeight}px`;
			document.body.appendChild(fullPageElem);

			let pendingRefresh: number | null = null;
			this.refreshSize = () => {
				if (pendingRefresh) return;
				pendingRefresh = window.setTimeout(() => {
					if (this.isExcluded) return;
					fullPageElem.style.height = "0";
					fullPageElem.style.height = `${this.root.scrollHeight}px`;
					pendingRefresh = null;
				}, 500);
			};

			setTimeout(this.refreshSize, 10);

			this.addEvent("resize", this.refreshSize as EventListener);

			const config: MutationObserverInit = {
				attributes: true,
				childList: true,
				characterData: false,
			};

			this.observer = new MutationObserver(this.refreshSize);
			this.observer.observe(body, config);

			if (this.root.offsetHeight <= windowHeight) {
				const clearfix = document.createElement("div");
				clearfix.style.clear = "both";
				body.appendChild(clearfix);
			}
		}

		if (!this.options.fixedBackground && !this.isExcluded) {
			body.style.backgroundAttachment = "scroll";
			html.style.backgroundAttachment = "scroll";
		}
	}

	private cleanup(): void {
		if (this.observer) {
			this.observer.disconnect();
		}
		this.removeEvent(this.wheelEvent, this.boundWheel as unknown as EventListener);
		this.removeEvent("mousedown", this.boundMousedown as unknown as EventListener);
		this.removeEvent("keydown", this.boundKeydown as unknown as EventListener);
		if (this.refreshSize) {
			this.removeEvent("resize", this.refreshSize as unknown as EventListener);
		}
		this.removeEvent("load", this.boundInit as unknown as EventListener);
	}

	private scrollArray(elem: Element, left: number, top: number): void {
		this.directionCheck(left, top);

		if (this.options.accelerationMax !== 1) {
			const now = Date.now();
			const elapsed = now - this.lastScroll;
			if (elapsed < this.options.accelerationDelta) {
				let factor = (1 + 50 / elapsed) / 2;
				if (factor > 1) {
					factor = Math.min(factor, this.options.accelerationMax);
					left *= factor;
					top *= factor;
				}
			}
			this.lastScroll = Date.now();
		}

		this.que.push({
			x: left,
			y: top,
			lastX: left < 0 ? 0.99 : -0.99,
			lastY: top < 0 ? 0.99 : -0.99,
			start: Date.now(),
		});

		if (this.pending) {
			return;
		}

		const scrollRoot = this.getScrollRoot();
		const isWindowScroll = elem === scrollRoot || elem === document.body;

		if (elem.$scrollBehavior == null && this.isScrollBehaviorSmooth(elem)) {
			elem.$scrollBehavior = (elem as HTMLElement).style.scrollBehavior;
			(elem as HTMLElement).style.scrollBehavior = "auto";
		}

		const step = () => {
			const now = Date.now();
			let scrollX = 0;
			let scrollY = 0;

			for (let i = 0; i < this.que.length; i++) {
				const item = this.que[i];
				const elapsed = now - item.start;
				const finished = elapsed >= this.options.animationTime;

				let position = finished ? 1 : elapsed / this.options.animationTime;

				if (this.options.pulseAlgorithm) {
					position = this.pulse(position);
				}

				const x = (item.x * position - item.lastX) >> 0;
				const y = (item.y * position - item.lastY) >> 0;

				scrollX += x;
				scrollY += y;

				item.lastX += x;
				item.lastY += y;

				if (finished) {
					this.que.splice(i, 1);
					i--;
				}
			}

			if (isWindowScroll) {
				window.scrollBy(scrollX, scrollY);
			} else {
				if (scrollX) (elem as HTMLElement).scrollLeft += scrollX;
				if (scrollY) (elem as HTMLElement).scrollTop += scrollY;
			}

			if (!left && !top) {
				this.que = [];
			}

			if (this.que.length) {
				this.requestFrame(step, elem, 1000 / this.options.frameRate + 1);
			} else {
				this.pending = false;
				if (elem.$scrollBehavior != null) {
					(elem as HTMLElement).style.scrollBehavior = elem.$scrollBehavior;
					elem.$scrollBehavior = null;
				}
			}
		};

		this.requestFrame(step, elem, 0);
		this.pending = true;
	}

	private wheel(event: WheelEvent): boolean {
		if (!this.initDone) {
			this.init();
		}

		const evt = event as WheelDeltaEvent;
		const target = event.target as Element;

		if (event.defaultPrevented || event.ctrlKey) {
			return true;
		}

		if (
			this.isNodeName(this.activeElement, "embed") ||
			(this.isNodeName(target, "embed") && /\.pdf/i.test((target as HTMLEmbedElement).src)) ||
			this.isNodeName(this.activeElement, "object") ||
			(target as Element & { shadowRoot?: ShadowRoot | null }).shadowRoot
		) {
			return true;
		}

		let deltaX = -(evt.wheelDeltaX ?? 0) || event.deltaX || 0;
		let deltaY = -(evt.wheelDeltaY ?? 0) || event.deltaY || 0;

		if (this.isMac) {
			if (evt.wheelDeltaX && this.isDivisible(evt.wheelDeltaX, 120)) {
				deltaX = -120 * (evt.wheelDeltaX / Math.abs(evt.wheelDeltaX));
			}
			if (evt.wheelDeltaY && this.isDivisible(evt.wheelDeltaY, 120)) {
				deltaY = -120 * (evt.wheelDeltaY / Math.abs(evt.wheelDeltaY));
			}
		}

		if (!deltaX && !deltaY) {
			deltaY = -(evt.wheelDelta ?? 0) || 0;
		}

		if (event.deltaMode === 1) {
			deltaX *= 40;
			deltaY *= 40;
		}

		let overflowing = this.overflowingAncestor(target);

		// 🛑 **Una lista con scroll propio que llega a su final pasa la rueda al
		// siguiente scroll de arriba** (09/10/2026). La librería original elige el
		// primer ancestro que desborda y se queda con él SIEMPRE: con una lista
		// dentro de una página (el muro de Novedades, 4 entradas de alto, más alto
		// que la ventana) la rueda se gastaba en la lista, que ya estaba al final,
		// y la página no bajaba nunca: las últimas entradas quedaban inalcanzables
		// con el ratón. El navegador, sin esto, sí encadena el scroll; aquí lo
		// hacíamos nosotros al interceptar el evento.
		// Solo en vertical y mientras la rueda sea sobre todo vertical.
		for (
			let saltos = 0;
			overflowing &&
			saltos < 20 &&
			Math.abs(deltaY) >= Math.abs(deltaX) &&
			!this.puedeMoverseY(overflowing, deltaY);
			saltos++
		) {
			const padre: Element | null = overflowing.parentElement;
			overflowing = padre ? this.overflowingAncestor(padre) : null;
		}

		if (!overflowing) {
			if (this.isFrame && this.isChrome) {
				Object.defineProperty(event, "target", { value: window.frameElement });
				const parentWindow = parent as unknown as ParentWheelTarget;
				return parentWindow.wheel ? parentWindow.wheel(event) : true;
			}
			return true;
		}

		if (this.isTouchpad(deltaY)) {
			return true;
		}

		if (Math.abs(deltaX) > 1.2) {
			deltaX *= this.options.stepSize / 120;
		}
		if (Math.abs(deltaY) > 1.2) {
			deltaY *= this.options.stepSize / 120;
		}

		this.scrollArray(overflowing, deltaX, deltaY);
		event.preventDefault();
		this.scheduleClearCache();
		return false;
	}

	private keydown(event: KeyboardEvent): boolean {
		const target = event.target as Element;
		const modifier =
			event.ctrlKey ||
			event.altKey ||
			event.metaKey ||
			(event.shiftKey && event.keyCode !== this.key.spacebar);

		if (this.activeElement && !document.body.contains(this.activeElement)) {
			this.activeElement = document.activeElement as HTMLElement;
		}

		const inputNodeNames = /^(textarea|select|embed|object)$/i;
		const buttonTypes = /^(button|submit|radio|checkbox|file|color|image)$/i;
		if (
			event.defaultPrevented ||
			inputNodeNames.test(target.nodeName) ||
			(this.isNodeName(target, "input") && !buttonTypes.test((target as HTMLInputElement).type)) ||
			this.isNodeName(this.activeElement, "video") ||
			this.isInsideYoutubeVideo(event) ||
			(target as HTMLElement).isContentEditable ||
			modifier
		) {
			return true;
		}

		if (
			(this.isNodeName(target, "button") ||
				(this.isNodeName(target, "input") &&
					buttonTypes.test((target as HTMLInputElement).type))) &&
			event.keyCode === this.key.spacebar
		) {
			return true;
		}

		if (
			this.isNodeName(target, "input") &&
			(target as HTMLInputElement).type === "radio" &&
			this.arrowKeys[event.keyCode]
		) {
			return true;
		}

		let shift: number;
		let x = 0;
		let y = 0;
		const overflowing = this.activeElement ? this.overflowingAncestor(this.activeElement) : null;

		if (!overflowing) {
			if (this.isFrame && this.isChrome) {
				const parentWindow = parent as unknown as ParentWheelTarget;
				return parentWindow.keydown ? parentWindow.keydown(event) : true;
			}
			return true;
		}

		let clientHeight = (overflowing as HTMLElement).clientHeight;
		if (overflowing === document.body) {
			clientHeight = window.innerHeight;
		}

		switch (event.keyCode) {
			case this.key.up:
				y = -this.options.arrowScroll;
				break;
			case this.key.down:
				y = this.options.arrowScroll;
				break;
			case this.key.spacebar:
				shift = event.shiftKey ? 1 : -1;
				y = -shift * clientHeight * 0.9;
				break;
			case this.key.pageup:
				y = -clientHeight * 0.9;
				break;
			case this.key.pagedown:
				y = clientHeight * 0.9;
				break;
			case this.key.home: {
				let homeOverflowing: Element = overflowing;
				if (overflowing === document.body && document.scrollingElement) {
					homeOverflowing = document.scrollingElement;
				}
				y = -(homeOverflowing as HTMLElement).scrollTop;
				break;
			}
			case this.key.end: {
				const scroll =
					(overflowing as HTMLElement).scrollHeight - (overflowing as HTMLElement).scrollTop;
				const scrollRemaining = scroll - clientHeight;
				y = scrollRemaining > 0 ? scrollRemaining + 10 : 0;
				break;
			}
			case this.key.left:
				x = -this.options.arrowScroll;
				break;
			case this.key.right:
				x = this.options.arrowScroll;
				break;
			default:
				return true;
		}

		this.scrollArray(overflowing, x, y);
		event.preventDefault();
		this.scheduleClearCache();
		return false;
	}

	private mousedown(event: MouseEvent): void {
		this.activeElement = event.target as HTMLElement;
	}

	private uniqueID(el: Element): number {
		// 🛑 `=== undefined`, NO `||` (que es lo que había). `uniqueIDCounter`
		// arranca en 0, así que el PRIMER elemento numerado recibe el id `0`, que
		// es falsy: con `||` se le asignaba un id NUEVO en cada llamada, su
		// entrada de caché no se encontraba jamás y `cacheX`/`cacheY` crecían sin
		// tope mientras se hacía scroll. Salió al mirar el único error que dejaba
		// `biome check` en el repo (`noAssignInExpressions`, 23/08/2026): la regla
		// existe precisamente porque una asignación escondida dentro de una
		// expresión no se lee, y aquí escondía un fallo real.
		if (el.uniqueID === undefined) {
			el.uniqueID = this.uniqueIDCounter++;
		}
		return el.uniqueID;
	}

	private scheduleClearCache(): void {
		if (this.clearCacheTimer) {
			clearTimeout(this.clearCacheTimer);
		}
		this.clearCacheTimer = window.setTimeout(() => {
			this.cacheX = {};
			this.cacheY = {};
			this.smoothBehaviorForElement = {};
		}, 1000);
	}

	private setCache(elems: Element[], overflowing: Element, x: boolean): Element {
		const cache = x ? this.cacheX : this.cacheY;
		for (let i = elems.length; i--; ) {
			cache[this.uniqueID(elems[i])] = overflowing;
		}
		return overflowing;
	}

	private getCache(el: Element, x: boolean): Element | undefined {
		return (x ? this.cacheX : this.cacheY)[this.uniqueID(el)];
	}

	private overflowingAncestor(el: Element | null): Element | null {
		if (!el) return null;
		const elems: Element[] = [];
		const body = document.body;
		const rootScrollHeight = this.root.scrollHeight;

		let current: Element | null = el;
		while (current) {
			const cached = this.getCache(current, false);
			if (cached) {
				return this.setCache(elems, cached, false);
			}
			elems.push(current);
			if (rootScrollHeight === (current as HTMLElement).scrollHeight) {
				const topOverflowsNotHidden =
					this.overflowNotHidden(this.root) && this.overflowNotHidden(body);
				const isOverflowCSS = topOverflowsNotHidden || this.overflowAutoOrScroll(this.root);
				if (
					(this.isFrame && this.isContentOverflowing(this.root)) ||
					(!this.isFrame && isOverflowCSS)
				) {
					return this.setCache(elems, this.getScrollRoot(), false);
				}
			} else if (this.isContentOverflowing(current) && this.overflowAutoOrScroll(current)) {
				return this.setCache(elems, current, false);
			}
			current = current.parentElement;
		}

		return null;
	}

	private puedeMoverseY(el: Element, deltaY: number): boolean {
		const e = el as HTMLElement;
		const alto = el === this.getScrollRoot() ? window.innerHeight : e.clientHeight;
		return puedeMoverseY(e.scrollTop, alto, e.scrollHeight, deltaY);
	}

	private isContentOverflowing(el: Element): boolean {
		return (el as HTMLElement).clientHeight + 10 < (el as HTMLElement).scrollHeight;
	}

	private overflowNotHidden(el: Element): boolean {
		const overflow = getComputedStyle(el as HTMLElement, "").getPropertyValue("overflow-y");
		return overflow !== "hidden";
	}

	private overflowAutoOrScroll(el: Element): boolean {
		const overflow = getComputedStyle(el as HTMLElement, "").getPropertyValue("overflow-y");
		return overflow === "scroll" || overflow === "auto";
	}

	private isScrollBehaviorSmooth(el: Element): boolean {
		const id = this.uniqueID(el);
		if (this.smoothBehaviorForElement[id] == null) {
			const scrollBehavior = getComputedStyle(el as HTMLElement, "").getPropertyValue(
				"scroll-behavior",
			);
			this.smoothBehaviorForElement[id] = scrollBehavior === "smooth";
		}
		return !!this.smoothBehaviorForElement[id];
	}

	private addEvent(type: string, fn: EventListener, arg?: boolean | AddEventListenerOptions): void {
		window.addEventListener(type, fn, arg || false);
	}

	private removeEvent(type: string, fn: EventListener, arg?: boolean | EventListenerOptions): void {
		window.removeEventListener(type, fn, arg || false);
	}

	private isNodeName(el: Element | null, tag: string): boolean {
		return !!el && (el.nodeName || "").toLowerCase() === tag.toLowerCase();
	}

	private directionCheck(x: number, y: number): void {
		const newX = x > 0 ? 1 : -1;
		const newY = y > 0 ? 1 : -1;
		if (this.direction.x !== newX || this.direction.y !== newY) {
			this.direction.x = newX;
			this.direction.y = newY;
			this.que = [];
			this.lastScroll = 0;
		}
	}

	private isTouchpad(deltaY: number): boolean {
		if (!deltaY) return false;
		if (!this.deltaBuffer.length) {
			this.deltaBuffer = [deltaY, deltaY, deltaY];
		}
		deltaY = Math.abs(deltaY);
		this.deltaBuffer.push(deltaY);
		this.deltaBuffer.shift();
		if (this.deltaBufferTimer) {
			clearTimeout(this.deltaBufferTimer);
		}
		this.deltaBufferTimer = window.setTimeout(() => {
			try {
				localStorage.SS_deltaBuffer = this.deltaBuffer.join(",");
			} catch {
				// Ignore quota / privacy mode errors
			}
		}, 1000);
		const dpiScaledWheelDelta = deltaY > 120 && this.allDeltasDivisibleBy(deltaY);
		const tp =
			!this.allDeltasDivisibleBy(120) && !this.allDeltasDivisibleBy(100) && !dpiScaledWheelDelta;
		if (deltaY < 50) return true;
		return tp;
	}

	private isDivisible(n: number, divisor: number): boolean {
		return Math.floor(n / divisor) === n / divisor;
	}

	private allDeltasDivisibleBy(divisor: number): boolean {
		return (
			this.isDivisible(this.deltaBuffer[0], divisor) &&
			this.isDivisible(this.deltaBuffer[1], divisor) &&
			this.isDivisible(this.deltaBuffer[2], divisor)
		);
	}

	private isInsideYoutubeVideo(event: KeyboardEvent): boolean {
		let elem: Element | null = event.target as Element;
		let isControl = false;
		if (document.URL.indexOf("www.youtube.com/watch") !== -1) {
			while (elem) {
				isControl = !!elem.classList?.contains("html5-video-controls");
				if (isControl) break;
				elem = elem.parentNode as Element | null;
			}
		}
		return isControl;
	}

	private requestFrame(callback: () => void, _element: Element, delay?: number): void {
		const raf =
			window.requestAnimationFrame ||
			(window as Window & { webkitRequestAnimationFrame?: typeof requestAnimationFrame })
				.webkitRequestAnimationFrame ||
			((cb: FrameRequestCallback) => window.setTimeout(cb as () => void, delay || 1000 / 60));
		raf(callback);
	}

	private getScrollRoot(): Element {
		const scrollRoot = document.scrollingElement;
		if (scrollRoot) {
			return scrollRoot;
		}

		const dummy = document.createElement("div");
		dummy.style.cssText = "height:10000px;width:1px;";
		document.body.appendChild(dummy);
		const bodyScrollTop = document.body.scrollTop;
		window.scrollBy(0, 3);
		let root: Element;
		if (document.body.scrollTop !== bodyScrollTop) {
			root = document.body;
		} else {
			root = document.documentElement;
		}
		window.scrollBy(0, -3);
		document.body.removeChild(dummy);
		return root;
	}

	/**
	 * Viscous fluid with a pulse for part and decay for the rest.
	 * Michael Herf, http://stereopsis.com/stopping/
	 */
	private pulse_(x: number): number {
		let val: number;
		let start: number;
		let expx: number;

		x = x * this.options.pulseScale;
		if (x < 1) {
			val = x - (1 - Math.exp(-x));
		} else {
			start = Math.exp(-1);
			x -= 1;
			expx = 1 - Math.exp(-x);
			val = start + expx * (1 - start);
		}
		return val * this.options.pulseNormalize;
	}

	private pulse(x: number): number {
		if (x >= 1) return 1;
		if (x <= 0) return 0;

		if (this.options.pulseNormalize === 1) {
			this.options.pulseNormalize /= this.pulse_(1);
		}
		return this.pulse_(x);
	}

	public static init(optionsToSet?: Partial<SmoothScrollOptions>): SmoothScroll {
		return SmoothScroll.getInstance(optionsToSet);
	}

	public updateOptions(optionsToSet: Partial<SmoothScrollOptions>): void {
		this.setOptions(optionsToSet);
	}

	public isInitialized(): boolean {
		return this.initDone;
	}

	public getOptions(): SmoothScrollOptions {
		return { ...this.options };
	}
}

if (typeof window !== "undefined") {
	if (window.SmoothScrollOptions) {
		SmoothScroll.init(window.SmoothScrollOptions);
	}
	window.SmoothScroll = SmoothScroll;
}
