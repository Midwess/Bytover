import {Toaster} from "react-hot-toast";

export default function AppToaster() {
    return (
        <Toaster
            position="bottom-center"
            reverseOrder={false}
            gutter={8}
            containerClassName=""
            containerStyle={{}}
            toastOptions={{
                // Define default options
                className: '',
                duration: 4000,
                style: {
                    background: 'hsl(var(--color-muted-foreground))',
                    color: 'hsl(var(--foreground))',
                    border: '2px solid var(--border)',
                    borderRadius: '8px',
                    padding: '16px',
                    fontSize: '14px',
                    fontWeight: '500',
                    boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
                    backdropFilter: 'blur(48px)',
                },
                // Custom styles for different toast types
                success: {
                    style: {
                        background: 'hsl(var(--background))',
                        color: 'hsl(var(--foreground))',
                        border: '1px solid hsl(142.1 76.2% 36.3%)',
                        boxShadow: '0 10px 15px -3px rgba(34, 197, 94, 0.1), 0 4px 6px -2px rgba(34, 197, 94, 0.05)',
                    },
                    iconTheme: {
                        primary: 'hsl(142.1 76.2% 36.3%)',
                        secondary: 'hsl(var(--background))',
                    },
                },
                error: {
                    style: {
                        background: 'hsl(var(--background))',
                        color: 'hsl(var(--foreground))',
                        border: '1px solid hsl(0 84.2% 60.2%)',
                        boxShadow: '0 10px 15px -3px rgba(239, 68, 68, 0.1), 0 4px 6px -2px rgba(239, 68, 68, 0.05)',
                    },
                    iconTheme: {
                        primary: 'hsl(0 84.2% 60.2%)',
                        secondary: 'hsl(var(--background))',
                    },
                },
                loading: {
                    style: {
                        background: 'hsl(var(--background))',
                        color: 'hsl(var(--foreground))',
                        border: '1px solid hsl(var(--border))',
                        boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
                    },
                    iconTheme: {
                        primary: 'hsl(var(--primary))',
                        secondary: 'hsl(var(--background))',
                    },
                },
            }}
        />
    );
}