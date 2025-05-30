//
//  App.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 28/1/25.
//

import SwiftUI
import SharedTypes
import ToastUI
import QuickLook

@main
struct Main: App {
    @StateObject private var core = Core()
    @State private var quicklook_url: URL?
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .onOpenURL(perform: {url in
                    Task {
                        await core.update(.authentication(.onRedirected(url: url.absoluteString)))
                    }
                })
                .quickLookPreview($quicklook_url)
                .environmentObject(core)
                .preferredColorScheme(.dark)
                .task {
                    await core.update(AppEvent.environment(.appLaunched))
                }
                .onReceive(core.quicklook_url, perform: {value in
                    quicklook_url = value
                })
        }
    }
}
