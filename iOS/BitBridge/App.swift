//
//  App.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 28/1/25.
//

import SwiftUI
import SharedTypes

@main
struct Main: App {
    @StateObject private var core = Core()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .onOpenURL(perform: {url in
                    Task {
                        await core.update(.authentication(.onRedirected(url: url.absoluteString)))
                    }
                })
                .environmentObject(core)
                .preferredColorScheme(.dark)
                .task {
                    print("Firing app launched")
                    await core.update(AppEvent.environment(.appLaunched))
                }
        }
    }
}
