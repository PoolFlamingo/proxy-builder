import { Slider as SliderPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

/** Deslizador de shadcn (Radix), con un solo valor o un rango. */
function Slider({
	className,
	defaultValue,
	value,
	min = 0,
	max = 100,
	...props
}: React.ComponentProps<typeof SliderPrimitive.Root>) {
	const valores = Array.isArray(value)
		? value
		: Array.isArray(defaultValue)
			? defaultValue
			: [min, max];
	return (
		<SliderPrimitive.Root
			data-slot="slider"
			defaultValue={defaultValue}
			value={value}
			min={min}
			max={max}
			className={cn(
				"relative flex w-full touch-none items-center select-none data-[disabled]:opacity-50",
				className,
			)}
			{...props}
		>
			<SliderPrimitive.Track
				data-slot="slider-track"
				className="relative h-1.5 w-full grow overflow-hidden rounded-full bg-muted"
			>
				<SliderPrimitive.Range data-slot="slider-range" className="absolute h-full bg-primary" />
			</SliderPrimitive.Track>
			{valores.map((_, i) => (
				<SliderPrimitive.Thumb
					data-slot="slider-thumb"
					key={i}
					className="block size-3.5 shrink-0 rounded-full border border-primary bg-background shadow-sm ring-ring/50 transition-[color,box-shadow] hover:ring-4 focus-visible:ring-4 focus-visible:outline-hidden"
				/>
			))}
		</SliderPrimitive.Root>
	);
}

export { Slider };
