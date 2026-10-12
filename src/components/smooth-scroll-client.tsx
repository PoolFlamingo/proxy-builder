import { useEffect } from "react";
import { useTheme } from "@/components/theme-provider";

/**
 * Monta el SmoothScroll de base/web (Balazs Galambosi) según el interruptor
 * de Apariencia → «Scroll suave». Sin Lenis.
 */
export function SmoothScrollClient() {
	const { smoothScroll } = useTheme();

	useEffect(() => {
		if (!smoothScroll) return;
		let cancelled = false;
		void import("@/lib/SmoothScroll").then(({ SmoothScroll }) => {
			if (cancelled) return;
			SmoothScroll.init();
		});
		return () => {
			cancelled = true;
			void import("@/lib/SmoothScroll").then(({ SmoothScroll }) => {
				SmoothScroll.destroy();
			});
		};
	}, [smoothScroll]);

	return null;
}
