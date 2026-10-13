import { type Events, on } from "@/lib/ipc";

/**
 * **Un trabajo largo de Rust** (resolver una lista, exportar un PDF): se
 * lanza, avisa del avance por un evento y termina con otro.
 *
 * 🛑 Se escucha ANTES de lanzar: un trabajo corto puede terminar antes de
 * que el `invoke` devuelva su id, y su evento final se perdería. Lo que llega
 * antes de saber el id se guarda y se reparte después.
 */
type ConId = { jobId: number };

export type Trabajo<D> = {
	jobId: number;
	/** Se cumple con la carga del evento final (también si fue un error). */
	fin: Promise<D>;
};

export async function lanzarTrabajo<P extends keyof Events, D extends keyof Events>(
	progreso: P,
	final: D,
	lanzar: () => Promise<number>,
	alProgreso: (p: Events[P]) => void,
): Promise<Trabajo<Events[D]>> {
	let id: number | null = null;
	const pendientesP: Events[P][] = [];
	const pendientesD: Events[D][] = [];
	let resolver: (d: Events[D]) => void = () => {};
	const fin = new Promise<Events[D]>((r) => {
		resolver = r;
	});

	const dejarP = await on(progreso, (p) => {
		if (id === null) pendientesP.push(p);
		else if ((p as ConId).jobId === id) alProgreso(p);
	});
	const dejarD = await on(final, (d) => {
		if (id === null) pendientesD.push(d);
		else if ((d as ConId).jobId === id) terminar(d);
	});
	const terminar = (d: Events[D]) => {
		dejarP();
		dejarD();
		resolver(d);
	};

	try {
		id = await lanzar();
	} catch (e) {
		dejarP();
		dejarD();
		throw e;
	}
	const mio = id;
	for (const p of pendientesP) if ((p as ConId).jobId === mio) alProgreso(p);
	const d = pendientesD.find((x) => (x as ConId).jobId === mio);
	if (d) terminar(d);
	return { jobId: mio, fin };
}
