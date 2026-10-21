import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { Aviso } from "@/bindings/Aviso";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";
import { useLang } from "@/components/language-provider";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { aplicarAlDeck, type Grupo, numerosUsados, quitarDiseno } from "./grupos";

const CLAVE_COLECCION = "proxybuilder.disenar.coleccion";

/** Cuánto sin cambios antes de volver a pintar la vista previa (Pablo, 20/10/2026):
 *  repintar a cada letra al escribir molesta. */
const ESPERA_VISTA_MS = 1000;

/** La última colección usada para los borradores nuevos (en este equipo). */
export function coleccionRecordada(): string | null {
	try {
		return localStorage.getItem(CLAVE_COLECCION);
	} catch {
		return null;
	}
}

export function recordarColeccion(id: string | null) {
	try {
		if (id) localStorage.setItem(CLAVE_COLECCION, id);
		else localStorage.removeItem(CLAVE_COLECCION);
	} catch {
		/* sin almacenamiento */
	}
}

/**
 * El diseño que se está editando: el de la carta si ya lo tiene; si no, un
 * borrador que pide a Rust (datos de Scryfall en inglés y español). Vive aquí
 * hasta «Guardar»: entonces Rust pinta la miniatura y la carta va al deck con
 * un solo `setCartas` (deshacer sigue funcionando).
 *
 * La vista previa se registra en Rust cuando se deja de tocar el diseño un
 * segundo (`ESPERA_VISTA_MS`; al cambiar de carta, cara o idioma, al
 * momento) y los avisos (texto que no cabe, símbolos que faltan…) con 400 ms.
 */
export function useBorrador(
	grupo: Grupo | null,
	cara: number,
	lang: DesignLang,
	/** La copia del grupo que se edita (su posición). */
	copia: number,
	/** Cambia cuando cambian colecciones, letras o packs: se vuelve a pintar. */
	recursos: unknown,
) {
	const { mensaje } = useLang();
	const [design, setDesign] = useState<CardDesign | null>(null);
	const [original, setOriginal] = useState<CardDesign | null>(null);
	const [cargando, setCargando] = useState(false);
	// La huella registrada y el diseño del que es (para saber si está al día).
	const [huella, setHuella] = useState<{ h: string; de: CardDesign } | null>(null);
	const [avisos, setAvisos] = useState<Aviso[]>([]);
	const [guardando, setGuardando] = useState(false);
	// Sube al quitar el diseño: la carta vuelve a pedir un borrador nuevo.
	const [recarga, setRecarga] = useState(0);
	const principal = grupo?.cartas[copia] ?? grupo?.cartas[0] ?? null;
	const idPrincipal = principal?.id ?? null;
	const clave = grupo?.clave ?? null;
	// Para no pisar un borrador con la respuesta de una carta anterior.
	const turno = useRef(0);

	// Al cambiar de carta: su diseño, o un borrador nuevo.
	useEffect(() => {
		const mio = ++turno.current;
		setHuella(null);
		setAvisos([]);
		if (!grupo || !principal || !grupo.scryfallId) {
			setDesign(null);
			setOriginal(null);
			return;
		}
		if (principal.design) {
			setDesign(principal.design);
			setOriginal(principal.design);
			return;
		}
		setDesign(null);
		setOriginal(null);
		setCargando(true);
		const cartas = useDeck.getState().deck?.cards ?? [];
		call("design_draft", {
			card: principal,
			usados: numerosUsados(cartas),
			collection: coleccionRecordada(),
			lang,
		})
			.then((b) => {
				if (turno.current !== mio) return;
				setDesign(b.design);
				// Un borrador recién hecho no es un cambio: «Descartar» vuelve a él.
				setOriginal(b.design);
				for (const a of b.avisos) setAvisos((v) => [...v, a]);
			})
			.catch((e) => toast.error(mensaje(e)))
			.finally(() => turno.current === mio && setCargando(false));
	}, [clave, idPrincipal, recarga]);

	// Si el diseño de la carta cambia por fuera (borradores en bloque,
	// deshacer, renumerar) y aquí no hay nada sin guardar, se toma el nuevo.
	const deFuera = principal?.design;
	const sinTocar = JSON.stringify(design) === JSON.stringify(original);
	useEffect(() => {
		if (!deFuera || !sinTocar) return;
		if (JSON.stringify(deFuera) === JSON.stringify(original)) return;
		setDesign(deFuera);
		setOriginal(deFuera);
	}, [deFuera]);

	// Vista previa: se registra al pasar `ESPERA_VISTA_MS` sin cambios (al
	// momento si es otra carta, cara, idioma o recursos); si llegan varios
	// mientras hay uno en marcha, solo el último.
	const registro = useRef<{ enVuelo: boolean; pendiente: (() => void) | null }>({
		enVuelo: false,
		pendiente: null,
	});
	const ultimoContexto = useRef<unknown[] | null>(null);
	useEffect(() => {
		if (!design) return;
		const mio = turno.current;
		const r = registro.current;
		const contexto = [turno.current, cara, lang, recursos];
		const antes = ultimoContexto.current;
		ultimoContexto.current = contexto;
		const mismo = !!antes && antes.every((v, i) => Object.is(v, contexto[i]));
		const pedir = () => {
			r.enVuelo = true;
			call("design_register", { design, cara, lang })
				.then((h) => turno.current === mio && setHuella({ h, de: design }))
				.catch((e) => toast.error(mensaje(e)))
				.finally(() => {
					r.enVuelo = false;
					const p = r.pendiente;
					r.pendiente = null;
					p?.();
				});
		};
		const lanzar = () => {
			if (r.enVuelo) r.pendiente = pedir;
			else pedir();
		};
		if (!mismo) {
			lanzar();
			return;
		}
		const id = setTimeout(lanzar, ESPERA_VISTA_MS);
		return () => clearTimeout(id);
	}, [design, cara, lang, mensaje, recursos]);

	// Avisos.
	useEffect(() => {
		if (!design) return;
		const id = setTimeout(() => {
			call("design_fit_report", { design, lang })
				.then(setAvisos)
				.catch(() => {});
		}, 400);
		return () => clearTimeout(id);
	}, [design, lang, recursos]);

	const cambiar = useCallback((f: (d: CardDesign) => CardDesign) => {
		setDesign((d) => (d ? f(d) : d));
	}, []);

	const guardar = useCallback(
		async (todas: boolean, listo?: boolean) => {
			if (!design || !grupo || !principal) return false;
			const deck = useDeck.getState().deck;
			if (!deck) return false;
			setGuardando(true);
			try {
				const d: CardDesign =
					listo === undefined ? design : { ...design, status: listo ? "ready" : "draft" };
				const nueva = await call("design_save", {
					card: principal,
					design: d,
					lang: deck.settings.designLang,
				});
				useDeck.getState().setCartas((cs) => aplicarAlDeck(cs, grupo, principal.id, nueva, todas));
				if (d.collection !== undefined) recordarColeccion(d.collection ?? null);
				setDesign(nueva.design ?? d);
				setOriginal(nueva.design ?? d);
				return true;
			} catch (e) {
				toast.error(mensaje(e));
				return false;
			} finally {
				setGuardando(false);
			}
		},
		[design, grupo, principal, mensaje],
	);

	const quitar = useCallback(
		(todas: boolean) => {
			if (!grupo || !principal) return;
			const ids = new Set(todas ? grupo.cartas.map((c) => c.id) : [principal.id]);
			useDeck.getState().setCartas((cs) => quitarDiseno(cs, ids));
			setRecarga((n) => n + 1);
		},
		[grupo, principal],
	);

	const cambiado = JSON.stringify(design) !== JSON.stringify(original);

	return {
		design,
		cambiar,
		cargando,
		huella: huella?.h ?? null,
		/** La huella es la del diseño de ahora (no de uno anterior). */
		alDia: !!design && huella?.de === design,
		avisos,
		guardar,
		guardando,
		quitar,
		cambiado,
		descartar: () => setDesign(original),
	};
}
