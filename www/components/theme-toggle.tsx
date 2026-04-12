"use client";

import { useEffect, useState } from "react";
import { LaptopMinimal, MoonStar, SunMedium } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

const themeOptions = [
  {
    value: "light",
    label: "Light",
    icon: SunMedium,
  },
  {
    value: "dark",
    label: "Dark",
    icon: MoonStar,
  },
  {
    value: "system",
    label: "System",
    icon: LaptopMinimal,
  },
] as const;

export function ThemeToggle() {
  const { resolvedTheme, setTheme, theme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const selectedTheme = mounted ? theme ?? "system" : "system";
  const activeTheme = mounted ? resolvedTheme ?? "light" : "light";

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          className="surface-quiet relative h-9 w-9 rounded-full border-border/70 shadow-none hover:border-primary/25 hover:bg-accent/40 hover:text-foreground"
          aria-label="Change color theme"
        >
          <SunMedium
            className={cn(
              "absolute size-4 transition-all duration-200",
              activeTheme === "dark"
                ? "scale-75 -rotate-90 opacity-0"
                : "scale-100 rotate-0 opacity-100",
            )}
          />
          <MoonStar
            className={cn(
              "absolute size-4 transition-all duration-200",
              activeTheme === "dark"
                ? "scale-100 rotate-0 opacity-100"
                : "scale-75 rotate-90 opacity-0",
            )}
          />
          <span className="sr-only">Change color theme</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="end"
        className="w-44 rounded-[1.1rem] border-border/70 p-2"
      >
        <DropdownMenuRadioGroup value={selectedTheme} onValueChange={setTheme}>
          {themeOptions.map((option) => {
            const Icon = option.icon;

            return (
              <DropdownMenuRadioItem
                key={option.value}
                value={option.value}
                className="rounded-xl px-3 py-2"
              >
                <Icon className="size-4" />
                <span>{option.label}</span>
              </DropdownMenuRadioItem>
            );
          })}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
