import React from "react";
import ReactDOM from "react-dom/client";
import { LanguageProvider } from "@/components/language-provider";
import { LimiteError } from "@/components/limite-error";
import { SmoothScrollClient } from "@/components/smooth-scroll-client";
import { ThemeProvider } from "@/components/theme-provider";
import { TooltipsNativos } from "@/components/tooltips-nativos";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
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

const raiz = document.getElementById("root");
if (!raiz) throw new Error("Falta el elemento #root en index.html");

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
