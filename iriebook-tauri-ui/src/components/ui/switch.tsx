import * as React from "react"
import { cn } from "../../lib/utils"

export interface SwitchProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'onChange'> {
  onCheckedChange?: (checked: boolean) => void;
}

const Switch = React.forwardRef<HTMLInputElement, SwitchProps>(
  ({ className, onCheckedChange, ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onCheckedChange?.(e.target.checked);
    };

    return (
      <label className="relative inline-flex items-center cursor-pointer">
        <input
          type="checkbox"
          className="sr-only peer"
          ref={ref}
          onChange={handleChange}
          {...props}
        />
        <div
          className={cn(
            "w-9 h-5 rounded-full border border-muted-foreground/50 bg-muted-foreground/35 shadow-inner transition-colors peer",
            "peer-checked:border-primary peer-checked:bg-primary",
            "after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-foreground after:shadow-sm",
            "after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:bg-primary-foreground",
            "peer-checked:after:translate-x-full",
            "peer-focus-visible:ring-2 peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2",
            "peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
            className
          )}
        />
      </label>
    )
  }
)
Switch.displayName = "Switch"

export { Switch }
