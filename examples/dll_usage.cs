using System;
using System.Runtime.InteropServices;

class Program
{
    [DllImport("path_server_lib.dll")]
    public static extern bool start_path_server(
        IntPtr mulPath,
        IntPtr uiPath,
        ushort httpPort
    );
    
    [DllImport("path_server_lib.dll")]
    public static extern void stop_path_server();    

    static void Main()
    {
        string mulPath = ".";
        string uiPath = "www/ui.html";
        ushort httpPort = 3000;

        IntPtr mulPathPtr = Marshal.StringToHGlobalAnsi(mulPath);
        IntPtr uiPathPtr = Marshal.StringToHGlobalAnsi(uiPath);

        bool result = start_path_server(mulPathPtr, uiPathPtr, httpPort);

        Marshal.FreeHGlobal(mulPathPtr);
        Marshal.FreeHGlobal(uiPathPtr);

        if (result)
        {
            Console.WriteLine("Path server started successfully, press enter to stop.");
            Console.ReadLine();
            stop_path_server();
        }
        else
        {
            Console.WriteLine("Failed to start path server.");
        }
    }
}