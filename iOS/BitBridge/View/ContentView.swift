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
    
    var body: some View {
        LandingView()
            .navigate(to: HomeView(), when: $core.is_signed_in)
            .overlay(FadingBackground())
    }
}
struct CounterView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
