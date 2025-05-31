//
//  ContentView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI
import SharedTypes

struct ContentView: View {
    @EnvironmentObject var core: Core
    @State var quicklook_path: LocalResourcePath? = nil
    
    var body: some View {
        ZStack {
            LandingView()
                .navigate(to: HomeView(), when: $core.isSignedIn)
                .overlay(FadingBackground())
            Alert()
            QuickLookSheet(path: $quicklook_path)
        }
        .onReceive(core.quicklook_path, perform: {value in
            quicklook_path = value
        })
    }
}
struct CounterView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
