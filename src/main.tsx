import React from "react";
import ReactDOM from "react-dom/client";
import { LanguageProvider, leerIdioma } from "@/components/language-provider";
import { LimiteError } from "@/components/limite-error";
import { SmoothScrollClient } from "@/components/smooth-scroll-client";
import { ThemeProvider } from "@/components/theme-provider";
import { TooltipsNativos } from "@/components/tooltips-nativos";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { cargarDiccionario } from "@/i18n/cargar";
import { ponerTemaAntesDeMontar } from "@/lib/tema-inicial";
import { prepararTemasInstalados } from "@/lib/temas-instalados";
import App from "./App";
import "./index.css";

// En producción no sale el menú contextual del WebView (Recargar, Inspeccionar)
// donde la app no pone uno propio. Va en burbuja y mirando `defaultPrevented`:
// si una carta abrió nuestro menú (Radix ya canceló el evento), no se toca.
if (!import.meta.env.DEV) {
	window.addEventListener("contextmenu", (e) => {
		if (!e.defaultPrevented) e.preventDefault();
	});
}

// Antes de montar: el `ThemeProvider` solo acepta un tema que esté en `THEMES`.
prepararTemasInstalados();
// Y el tema ya puesto en `<html>`: la ventana se enseña enseguida y el primer
// pintado tiene que salir con sus colores, no con los claros por defecto.
ponerTemaAntesDeMontar();

const raiz = document.getElementById("root");
if (!raiz) throw new Error("Falta el elemento #root en index.html");

// La ventana arranca oculta y con fondo oscuro (`tauri.conf.json`). Se enseña
// ya, sin esperar a que pinte: WebKitGTK no da fotogramas
// (`requestAnimationFrame`) a una ventana oculta y entonces salía a los 3
// segundos. No hay destello blanco porque el tema ya está puesto
// (`ponerTemaAntesDeMontar`). Rust también la enseña al cargar la página.
function ensenarVentana() {
	import("@tauri-apps/api/window")
		.then(({ getCurrentWindow }) => getCurrentWindow().show())
		.catch(() => {});
}

function montar(raiz: HTMLElement) {
	ReactDOM.createRoot(raiz).render(
		<React.StrictMode>
			{/* Un error de render nunca debe dejar la ventana en blanco. */}
			<LimiteError>
				<LanguageProvider>
					<ThemeProvider>
						<SmoothScrollClient />
						<TooltipProvider>
							<App />
							<Toaster position="bottom-right" />
							{/* Ningún `title` nativo llega a pintarse: se convierte en el Tooltip de la casa. */}
							<TooltipsNativos />
						</TooltipProvider>
					</ThemeProvider>
				</LanguageProvider>
			</LimiteError>
		</React.StrictMode>,
	);
}

// El diccionario del idioma elegido antes del primer pintado (el inglés ya va
// en el paquete): así la app no sale un instante en otro idioma. Si no carga,
// se monta igual y los textos caen al inglés.
void cargarDiccionario(leerIdioma()).finally(() => montar(raiz));
ensenarVentana();
