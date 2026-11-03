import { useFormato } from "@/lib/formato";

/**
 * **¿Es la mesa de un móvil?** Un teléfono, de pie o de lado (914 × 411 dp):
 * `useFormato` ya lo dice por el lado corto (`formato.ts`, regla `sw600dp`).
 * En la mesa significa un tablero a la vez, la barra de asientos, la mano y la
 * vida a pantalla completa y los diálogos también a pantalla completa.
 */
export function useMesaMovil(): boolean {
	return useFormato().tamano === "movil";
}
