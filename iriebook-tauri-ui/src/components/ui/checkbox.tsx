import * as React from "react"
import { Check } from "lucide-react"

import { cn } from "../../lib/utils"

export interface CheckboxProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'onChange'> {
  onCheckedChange?: (checked: boolean) => void;
}

const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, onCheckedChange, ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onCheckedChange?.(e.target.checked);
    };

    return (
      <div className="relative inline-flex items-center">
        <input
          type="checkbox"
          className={cn(
            "peer h-5 w-5 shrink-0 rounded-sm border-2 border-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
            "appearance-none bg-zinc-300 transition-colors hover:bg-zinc-200 checked:border-primary checked:bg-primary checked:hover:bg-primary",
            className
          )}
          ref={ref}
          onChange={handleChange}
          {...props}
        />
        <Check
          className={cn(
            "pointer-events-none absolute left-0 h-5 w-5 text-primary-foreground opacity-0 peer-checked:opacity-100"
          )}
          strokeWidth={3}
        />
      </div>
    )
  }
)
Checkbox.displayName = "Checkbox"

export { Checkbox }
