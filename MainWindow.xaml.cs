using System.Windows;
using System.Net.WebSockets;
using Microsoft.AspNetCore.Builder;
using System.IO;
using Microsoft.Extensions.FileProviders;
using Microsoft.Extensions.Logging;
using System.Windows.Interop;
using System.Diagnostics;

namespace toucca
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        SerialManager serialManager = new();

        public MainWindow()
        {
            this.WindowStartupLocation = WindowStartupLocation.CenterScreen;
            Task.Run(StartWebHost);
            Task.Run(serialManager.Start);
            Topmost = true;
            InitializeComponent();
        }

        private async void webView_Initialized(object sender, System.EventArgs e)
        {
            await webView.EnsureCoreWebView2Async(null);
            AutoLocateMercury();
            ExitWithMercury();
        }

        private void StartWebHost()
        {
            // serve content ftom web folder
            WebApplicationBuilder builder = WebApplication.CreateBuilder(Environment.GetCommandLineArgs());
            WebApplication webApplication = builder.Build();
            webApplication.UseWebSockets();
            webApplication.Use(
                async (ctx, next) =>
                {
                    if (ctx.WebSockets.IsWebSocketRequest && ctx.Request.Path == "/")
                    {
                        using WebSocket webSocket = await ctx.WebSockets.AcceptWebSocketAsync();
                        await HandleWebSocketClient(webSocket);
                    }
                    else
                    {
                        await next();
                    }
                });
            FileServerOptions options = new();
            options.FileProvider = new PhysicalFileProvider(Path.Combine(builder.Environment.ContentRootPath, "web"));
            options.EnableDirectoryBrowsing = false;
            webApplication.UseFileServer(options);
            webApplication.Run("http://127.0.0.1:25730");

        }

        private async void ExitWithMercury()
        {
            Process mercuryProc;
            while (true)
            {
                var procs = Process.GetProcessesByName("Mercury-Win64-Shipping");
                if (procs.Length != 0)
                {
                    mercuryProc = procs[0];
                    break;
                }
                await Task.Delay(1000);
            }
            await mercuryProc.WaitForExitAsync();
            App.Current.Shutdown();
        }

        private async void AutoLocateMercury()
        {
            while (true)
            {
                await Task.Delay(300);
                var pos = MercuryHelper.TryLocateMecury();
                if (pos.HasValue)
                {
                    var rect = pos.Value;
                    MercuryHelper.RECT currRect;
                    var hwnd = new WindowInteropHelper(this).Handle;
                    MercuryHelper.GetWindowRect(hwnd, out currRect);

                    if (currRect.Top != (int)rect.Top || currRect.Left != (int)rect.Left || currRect.Width != (int)rect.Width || currRect.Height != (int)rect.Height)
                    { 
                        Logger.Info("Repositioning window");
                        MercuryHelper.SetWindowRect(hwnd, rect);
                        webView.Reload();
                    }
                }
                
            }
        }

        private async Task HandleWebSocketClient(WebSocket webSocket)
        {
            byte[] currentState = new byte[30];
            byte[] buffer = new byte[30];
            WebSocketReceiveResult async;
            for (async = await webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None); !async.CloseStatus.HasValue; async = await webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None))
            {
                if (async.Count == 1 && buffer[0] == 71)
                    Array.Clear(buffer);
                else if (async.Count != 30)
                {
                    Logger.Warn($"Received {async.Count} bytes instead of 30");
                }
                for (int index1 = 0; index1 < 30; ++index1)
                {
                    if (buffer[index1] != currentState[index1])
                    {
                        byte num = buffer[index1];
                        for (int index2 = 0; index2 < 8; ++index2)
                        {
                            if ((num & (1 << index2)) != (currentState[index1] & (1 << index2)))
                                WriteKeyState((byte)Area2Area(index1 * 8 + index2), (num & (1 << index2)) > 0);
                        }
                        currentState[index1] = num;
                    }
                }
            }
            await webSocket.CloseAsync(async.CloseStatus.Value, async.CloseStatusDescription, CancellationToken.None);
        }


        private static int Area2Area(int fromArea) => fromArea >= 120 ? fromArea - 120 : fromArea + 120;

        private void WriteKeyState(byte key, bool enabled)
        {
            serialManager.SetTouch(key, enabled);
            serialManager.TouchEvent.Set();
        }
    }
}