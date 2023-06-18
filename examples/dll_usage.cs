using System;
using System.Runtime.InteropServices;

class Program
{
    [DllImport("path_server_lib.dll")]
    public static extern bool start_path_server_ex(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string dataPath,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string uiPath,
        ushort httpPort
    );


    [DllImport("path_server_lib.dll")]
    public static extern void stop_path_server();    

    static void Main()
    {
        string dataPath = ".";
        string uiFile = "www/ui.html";
        ushort httpPort = 3000;

        bool result = start_path_server_ex(dataPath, uiFile, httpPort);

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