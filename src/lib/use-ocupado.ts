import { useCallback, useRef, useState } from "react";

/**
 * Para los botones que lanzan algo que tarda (crear una sala, entrar,
 * invitar…): `ocupado` mientras dura, y `envolver(f)` no deja empezarlo otra
 * vez hasta que acaba (un doble clic no lo hace dos veces).
 */
export function useOcupado(): [boolean, <T>(f: () => Promise<T>) => Promise<T | undefined>] {
	const [ocupado, setOcupado] = useState(false);
	const enCurso = useRef(false);
	const envolver = useCallback(async <T>(f: () => Promise<T>) => {
		if (enCurso.current) return undefined;
		enCurso.current = true;
		setOcupado(true);
		try {
			return await f();
		} finally {
			enCurso.current = false;
			setOcupado(false);
		}
	}, []);
	return [ocupado, envolver];
}
