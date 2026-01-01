import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { Button } from "./ui/button";
import { CheckCircle2, AlertCircle, LogOut, Loader2, ExternalLink } from "lucide-react";

export function GoogleAuthPanel() {
  const { t } = useTranslation();
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check authentication status on mount
  useEffect(() => {
    checkAuthStatus();
  }, []);

  const checkAuthStatus = async () => {
    try {
      const result = await commands.googleCheckAuth();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      setIsAuthenticated(result.data);
    } catch (err) {
      console.error("Failed to check Google auth status:", err);
    }
  };

  const handleAuthenticate = async () => {
    setIsAuthenticating(true);
    setError(null);

    try {
      // Start auth flow - this will open the browser and await the code
      const result = await commands.googleAuthStart();
      if (result.status === "error") {
         // If "Authentication cancelled" is the error, we might want to handle it gracefully
         if (result.error.includes("cancelled")) {
             setIsAuthenticating(false);
             return;
         }
         throw new Error(result.error);
      }

      // If we get here, auth was successful
      setIsAuthenticated(true);
      setIsAuthenticating(false);
      setError(null);
    } catch (err) {
      setError(err as string);
      setIsAuthenticating(false);
    }
  };

  const handleCancel = async () => {
    try {
      await commands.googleAuthCancel();
      // The handleAuthenticate promise will reject with "Authentication cancelled"
    } catch (err) {
      console.error("Failed to cancel auth:", err);
    }
  };

  const handleLogout = async () => {
    try {
      const result = await commands.googleLogout();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      setIsAuthenticated(false);
    } catch (err) {
      setError(err as string);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-medium">{t("google.auth.title")}</h3>
        {isAuthenticated && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleLogout}
            className="h-8 text-muted-foreground hover:text-destructive"
          >
            <LogOut className="mr-2 h-4 w-4" />
            {t("google.auth.disconnect")}
          </Button>
        )}
      </div>

      {isAuthenticated ? (
        <div className="flex items-center gap-2 p-3 bg-green-50 dark:bg-green-950/20 text-green-700 dark:text-green-400 rounded-md border border-green-200 dark:border-green-900">
          <CheckCircle2 className="h-5 w-5" />
          <p className="font-medium">{t("google.auth.connected")}</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-center gap-2 p-3 bg-amber-50 dark:bg-amber-950/20 text-amber-700 dark:text-amber-400 rounded-md border border-amber-200 dark:border-amber-900">
            <AlertCircle className="h-5 w-5" />
            <p className="font-medium">{t("google.auth.notConnected")}</p>
          </div>

          {isAuthenticating ? (
            <div className="space-y-3">
                 <div className="flex items-center justify-center p-4 border border-dashed rounded-md bg-muted/50">
                    <div className="text-center space-y-2">
                        <Loader2 className="h-8 w-8 animate-spin mx-auto text-primary" />
                        <p className="text-sm font-medium">Browser opened...</p>
                        <p className="text-xs text-muted-foreground">Please sign in to Google to continue.</p>
                    </div>
                 </div>
                 <Button 
                    variant="outline" 
                    onClick={handleCancel}
                    className="w-full"
                 >
                    Cancel
                 </Button>
            </div>
          ) : (
            <Button
              onClick={handleAuthenticate}
              className="w-full"
            >
              <ExternalLink className="mr-2 h-4 w-4" />
              {t("google.auth.connect")}
            </Button>
          )}
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 text-destructive rounded-md text-sm">
          <p>{error}</p>
        </div>
      )}
    </div>
  );
}